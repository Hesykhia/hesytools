<script lang="ts">
  // The Quick Clicker as a compact side panel. It reads as a sentence; each
  // chip opens a labelled panel laid out like OP Auto Clicker's options.
  import { onDestroy, onMount } from "svelte";
  import {
    api,
    errorText,
    MOUSE_BUTTONS,
    onClickerState,
    type ClickerConfig,
    type HotkeyMode,
    type Settings,
  } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "../HotkeyButton.svelte";
  import Popover from "../ui/Popover.svelte";
  import Tabs from "../ui/Tabs.svelte";
  import Slider from "../ui/Slider.svelte";
  import Disclosure from "../ui/Disclosure.svelte";
  import ClickerOrb from "./ClickerOrb.svelte";

  let { settings = $bindable(), running = $bindable(false) }: { settings: Settings; running?: boolean } = $props();

  // svelte-ignore state_referenced_locally
  let cfg = $state<ClickerConfig>($state.snapshot(settings.clicker));
  cfg.double ??= false;

  // ── Persist edits (debounced) ────────────────────────────────────────────
  // Cleared number inputs come through as null; never send those to Rust.
  const num = (v: unknown, fallback: number, min = 0) => {
    const n = Number(v);
    return v === null || v === "" || !Number.isFinite(n) ? fallback : Math.max(min, n);
  };
  function sanitize(c: ClickerConfig): ClickerConfig {
    const out = { ...c, holdMs: num(c.holdMs, 0), jitterMs: num(c.jitterMs, 0), intervalMs: num(c.intervalMs, 100, 1) };
    if (out.position.kind === "fixed")
      out.position = { kind: "fixed", x: Math.round(num(out.position.x, 0, -1e6)), y: Math.round(num(out.position.y, 0, -1e6)) };
    if (out.stopAfter.kind === "clicks") out.stopAfter = { kind: "clicks", count: Math.round(num(out.stopAfter.count, 1, 1)) };
    if (out.stopAfter.kind === "seconds") out.stopAfter = { kind: "seconds", secs: num(out.stopAfter.secs, 1, 0.1) };
    return out;
  }

  let lastSaved = JSON.stringify(cfg);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const snap = JSON.stringify(cfg);
    if (snap === lastSaved) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(async () => {
      try {
        const plain = sanitize($state.snapshot(cfg));
        await api.setClickerConfig(plain);
        lastSaved = snap;
        settings.clicker = plain;
      } catch (e) {
        toast(errorText(e), "error");
      }
    }, 250);
  });

  // ── Interval as hours / mins / secs / ms, like OP Auto Clicker ───────────
  const UNITS = [
    { key: "h", label: "hours", ms: 3_600_000 },
    { key: "m", label: "mins", ms: 60_000 },
    { key: "s", label: "secs", ms: 1000 },
    { key: "ms", label: "milliseconds", ms: 1 },
  ] as const;
  function split(total: number): Record<string, number> {
    let rest = Math.round(total);
    const out: Record<string, number> = {};
    for (const u of UNITS) {
      out[u.key] = Math.floor(rest / u.ms);
      rest -= out[u.key] * u.ms;
    }
    return out;
  }
  let parts = $derived(split(cfg.intervalMs));
  function setPart(key: string, raw: string) {
    const v = Math.max(0, Math.floor(Number(raw) || 0));
    const next = { ...parts, [key]: v };
    const total = UNITS.reduce((sum, u) => sum + next[u.key] * u.ms, 0);
    cfg.intervalMs = Math.max(1, Math.min(total, 24 * 3_600_000));
  }
  const PRESETS = [
    { label: "1 ms", ms: 1 },
    { label: "10 ms", ms: 10 },
    { label: "50 ms", ms: 50 },
    { label: "100 ms", ms: 100 },
    { label: "1 s", ms: 1000 },
  ];

  // ── The sentence ──────────────────────────────────────────────────────────
  const BUTTON: Record<string, string> = {
    LeftMouse: "left",
    RightMouse: "right",
    MiddleMouse: "middle",
    Mouse4: "Mouse 4",
    Mouse5: "Mouse 5",
  };
  let actionText = $derived(
    cfg.action.kind === "mouse"
      ? `${cfg.double ? "Double " : ""}${cfg.double ? BUTTON[cfg.action.button] : BUTTON[cfg.action.button][0].toUpperCase() + BUTTON[cfg.action.button].slice(1)} click`
      : `Press ${cfg.action.key}`,
  );
  let rateText = $derived.by(() => {
    const p = parts;
    if (cfg.intervalMs < 1000) return `every ${p.ms} ms`;
    if (p.h === 0 && p.m === 0 && p.ms === 0) return p.s === 1 ? "every second" : `every ${p.s} seconds`;
    const bits = [p.h && `${p.h} h`, p.m && `${p.m} min`, p.s && `${p.s} s`, p.ms && `${p.ms} ms`].filter(Boolean);
    return `every ${bits.join(" ")}`;
  });
  let perSecond = $derived(Math.round((1000 / cfg.intervalMs) * 100) / 100);
  let whereText = $derived(cfg.position.kind === "current" ? "wherever my cursor is" : `at ${cfg.position.x}, ${cfg.position.y}`);
  let stopText = $derived(
    cfg.stopAfter.kind === "never"
      ? "until I stop it"
      : cfg.stopAfter.kind === "clicks"
        ? `${cfg.stopAfter.count} times`
        : `for ${cfg.stopAfter.secs} seconds`,
  );

  // ── Running state + live CPS ─────────────────────────────────────────────
  let clicks = $state(0);
  let blocked = $state(0);
  let cps = $state(0);
  let countdown = $state(0);
  let countdownTimer: ReturnType<typeof setInterval> | undefined;
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let last = { clicks: 0, at: 0 };
  let phase = $derived<"idle" | "countdown" | "running">(running ? "running" : countdown > 0 ? "countdown" : "idle");

  async function poll() {
    const s = await api.clickerStatus();
    const now = performance.now();
    if (s.clicks < last.clicks) last = { clicks: 0, at: now };
    if (last.at > 0 && now > last.at) {
      const inst = ((s.clicks - last.clicks) * 1000) / (now - last.at);
      cps = cps === 0 ? inst : cps * 0.6 + inst * 0.4;
    }
    last = { clicks: s.clicks, at: now };
    clicks = s.clicks;
    blocked = s.blocked;
    running = s.running;
  }

  function setRunning(r: boolean) {
    running = r;
    clearInterval(pollTimer);
    if (r) {
      cps = 0;
      last = { clicks: 0, at: performance.now() };
      pollTimer = setInterval(poll, 250);
    } else {
      poll();
    }
  }

  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await onClickerState(setRunning);
    setRunning((await api.clickerStatus()).running);
  });
  onDestroy(() => {
    unlisten?.();
    clearInterval(pollTimer);
    clearInterval(countdownTimer);
    clearTimeout(saveTimer);
  });

  // Starting from the orb waits 3s so you can move to the target first.
  // Otherwise a left-clicker would immediately click its own Stop button.
  async function startStop() {
    if (countdown > 0) {
      clearInterval(countdownTimer);
      countdown = 0;
      return;
    }
    if (running) {
      await api.clickerToggle().catch((e) => toast(errorText(e), "error"));
      return;
    }
    countdown = 3;
    countdownTimer = setInterval(async () => {
      if (countdown <= 1) {
        clearInterval(countdownTimer);
        await api.clickerToggle().catch((e) => toast(errorText(e), "error"));
        countdown = 0;
      } else countdown -= 1;
    }, 1000);
  }

  // ── Editors ───────────────────────────────────────────────────────────────
  let capturingKey = $state(false);
  async function captureKey() {
    capturingKey = true;
    try {
      const combo = await api.captureHotkey();
      if (!combo) return;
      const trigger = combo.split("+").pop()!;
      const mouse = MOUSE_BUTTONS.find((b) => b.value === trigger);
      cfg.action = mouse ? { kind: "mouse", button: mouse.value } : { kind: "key", key: trigger };
    } finally {
      capturingKey = false;
    }
  }

  let picking = $state(false);
  async function pick() {
    if (picking) return api.cancelPick();
    picking = true;
    try {
      const p = await api.pickPoint();
      if (p) cfg.position = { kind: "fixed", x: p[0], y: p[1] };
    } finally {
      picking = false;
    }
  }

  async function setHotkey(v: string | null) {
    try {
      settings = await api.setHotkey("clicker", v);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function setMode(mode: HotkeyMode) {
    await api.setClickerMode(mode);
    settings.clickerMode = mode;
  }
</script>

<section class="panel glass" class:live={running}>
  <header>
    <span class="eyebrow">Quick Clicker</span>
    {#if running}
      <span class="pill live"><span class="dot"></span>{clicks.toLocaleString()} {cfg.action.kind === "key" ? "presses" : "clicks"}</span>
    {:else if clicks > 0}
      <span class="pill">{clicks.toLocaleString()} last run</span>
    {/if}
  </header>

  <div class="orb-wrap"><ClickerOrb {phase} {cps} {countdown} onclick={startStop} size={150} /></div>

  <p class="sentence">
    <Popover width={320}>
      {#snippet trigger({ toggle, open })}
        <button class="chip" class:open onclick={toggle}>{actionText}</button>
      {/snippet}
      {#snippet children()}
        <div class="field">
          <span class="label">Mouse button</span>
          <div class="tiles">
            {#each MOUSE_BUTTONS as b}
              <button
                class="tile"
                class:on={cfg.action.kind === "mouse" && cfg.action.button === b.value}
                onclick={() => (cfg.action = { kind: "mouse", button: b.value })}>{b.label.replace(/ \(.*\)/, "")}</button
              >
            {/each}
          </div>
        </div>
        {#if cfg.action.kind === "mouse"}
          <div class="field">
            <span class="label">Click type</span>
            <Tabs
              size="s"
              value={cfg.double ? "double" : "single"}
              onchange={(v) => (cfg.double = v === "double")}
              options={[
                { value: "single", label: "Single" },
                { value: "double", label: "Double" },
              ]}
            />
          </div>
        {/if}
        <div class="field">
          <span class="label">Or a keyboard key</span>
          <button class="tile wide" class:on={cfg.action.kind === "key"} class:listening={capturingKey} onclick={captureKey}>
            {#if capturingKey}Press any key…{:else if cfg.action.kind === "key"}<kbd>{cfg.action.key}</kbd> click to change{:else}Choose a key…{/if}
          </button>
        </div>
      {/snippet}
    </Popover>

    <Popover width={340}>
      {#snippet trigger({ toggle, open })}
        <button class="chip" class:open onclick={toggle}>{rateText}</button>
      {/snippet}
      {#snippet children()}
        <div class="field">
          <span class="label">Click interval</span>
          <div class="hms">
            {#each UNITS as u}
              <label>
                <input type="number" min="0" value={parts[u.key]} onchange={(e) => setPart(u.key, e.currentTarget.value)} />
                <small>{u.label}</small>
              </label>
            {/each}
          </div>
        </div>
        <label class="check">
          <input
            type="checkbox"
            checked={cfg.jitterMs > 0}
            onchange={(e) => (cfg.jitterMs = e.currentTarget.checked ? Math.min(40, Math.round(cfg.intervalMs / 2)) || 1 : 0)}
          />
          Random offset ±
          <input class="small" type="number" min="0" bind:value={cfg.jitterMs} disabled={cfg.jitterMs === 0} />
          milliseconds
        </label>
        <div class="presets">
          {#each PRESETS as p}
            <button class="preset" class:on={cfg.intervalMs === p.ms} onclick={() => (cfg.intervalMs = p.ms)}>{p.label}</button>
          {/each}
        </div>
        <p class="note">= {perSecond} clicks per second. Some games cap how fast they read input.</p>
      {/snippet}
    </Popover>

    {#if cfg.action.kind === "mouse"}
      <Popover width={300}>
        {#snippet trigger({ toggle, open })}
          <button class="chip" class:open onclick={toggle}>{whereText}</button><span class="punct">,</span>
        {/snippet}
        {#snippet children()}
          <span class="label">Cursor position</span>
          <button class="option" class:on={cfg.position.kind === "current"} onclick={() => (cfg.position = { kind: "current" })}>
            <span class="radio"></span><span><b>Current location</b><small>Never moves your mouse</small></span>
          </button>
          <button
            class="option"
            class:on={cfg.position.kind === "fixed"}
            onclick={() => {
              if (cfg.position.kind !== "fixed") {
                cfg.position = { kind: "fixed", x: 0, y: 0 };
                pick();
              }
            }}
          >
            <span class="radio"></span><span><b>Pick location</b><small>Jumps there before every click</small></span>
          </button>
          {#if cfg.position.kind === "fixed"}
            <div class="coords">
              <label>X <input type="number" bind:value={cfg.position.x} /></label>
              <label>Y <input type="number" bind:value={cfg.position.y} /></label>
              <button class="btn small" class:picking onclick={pick}>{picking ? "Click anywhere…" : "Pick"}</button>
            </div>
          {/if}
        {/snippet}
      </Popover>
    {:else}<span class="punct">,</span>{/if}

    <Popover width={290}>
      {#snippet trigger({ toggle, open })}
        <button class="chip" class:open onclick={toggle}>{stopText}</button><span class="punct">.</span>
      {/snippet}
      {#snippet children()}
        <span class="label">Click repeat</span>
        <div
          class="option"
          class:on={cfg.stopAfter.kind === "clicks"}
          role="radio"
          tabindex="0"
          aria-checked={cfg.stopAfter.kind === "clicks"}
          onclick={() => cfg.stopAfter.kind !== "clicks" && (cfg.stopAfter = { kind: "clicks", count: 100 })}
          onkeydown={(e) => e.key === "Enter" && cfg.stopAfter.kind !== "clicks" && (cfg.stopAfter = { kind: "clicks", count: 100 })}
        >
          <span class="radio"></span>
          <span class="inline">
            Repeat
            {#if cfg.stopAfter.kind === "clicks"}
              <input class="small" type="number" min="1" bind:value={cfg.stopAfter.count} />
            {:else}<b>N</b>{/if}
            times
          </span>
        </div>
        <div
          class="option"
          class:on={cfg.stopAfter.kind === "seconds"}
          role="radio"
          tabindex="0"
          aria-checked={cfg.stopAfter.kind === "seconds"}
          onclick={() => cfg.stopAfter.kind !== "seconds" && (cfg.stopAfter = { kind: "seconds", secs: 60 })}
          onkeydown={(e) => e.key === "Enter" && cfg.stopAfter.kind !== "seconds" && (cfg.stopAfter = { kind: "seconds", secs: 60 })}
        >
          <span class="radio"></span>
          <span class="inline">
            Keep going for
            {#if cfg.stopAfter.kind === "seconds"}
              <input class="small" type="number" min="0.1" step="any" bind:value={cfg.stopAfter.secs} />
            {:else}<b>N</b>{/if}
            seconds
          </span>
        </div>
        <button class="option" class:on={cfg.stopAfter.kind === "never"} onclick={() => (cfg.stopAfter = { kind: "never" })}>
          <span class="radio"></span><span class="inline">Repeat until stopped</span>
        </button>
      {/snippet}
    </Popover>
  </p>

  <div class="hotkey-row">
    <HotkeyButton value={settings.clickerHotkey} onchange={setHotkey} />
    <span class="muted">{settings.clickerMode === "hold" ? "hold to click" : "start / stop"}</span>
    <span class="spacer"></span>
    <Tabs
      size="s"
      value={settings.clickerMode}
      onchange={setMode}
      options={[
        { value: "toggle", label: "Toggle" },
        { value: "hold", label: "Hold" },
      ]}
    />
  </div>

  {#if blocked > 0}
    <div class="warn">
      Windows blocked {blocked.toLocaleString()} clicks: that window runs as admin. Turn on admin mode in Settings.
    </div>
  {/if}

  <Disclosure title="Hold time">
    <div class="tune">
      <Slider value={cfg.holdMs} max={100} onchange={(v) => (cfg.holdMs = v)} />
      <span class="val">{Math.round(cfg.holdMs)} ms</span>
    </div>
    <p class="tune-hint">How long each press is held. Games often need 10–20 ms to notice a press.</p>
  </Disclosure>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 18px 20px 16px;
    transition:
      border-color 800ms var(--ease-in-out),
      box-shadow 800ms var(--ease-in-out);
  }
  .panel.live {
    border-color: rgba(61, 242, 176, 0.3);
    box-shadow: 0 0 50px -20px rgba(61, 242, 176, 0.4);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 22px;
  }
  .pill {
    font-size: 11.5px;
    color: var(--text-3);
    padding: 2px 9px;
    border-radius: 99px;
    border: 1px solid var(--stroke);
    font-variant-numeric: tabular-nums;
  }
  .pill.live {
    color: var(--mint);
    border-color: rgba(61, 242, 176, 0.35);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mint);
  }
  .orb-wrap {
    display: flex;
    justify-content: center;
  }
  .sentence {
    font-size: 17px;
    font-weight: 600;
    letter-spacing: -0.015em;
    line-height: 1.9;
    color: var(--text-2);
    text-align: center;
  }
  .chip {
    font-size: inherit;
    font-weight: 650;
    padding: 0 9px;
    margin: 0 1px;
    border-radius: 9px;
    color: var(--text);
    background: linear-gradient(180deg, rgba(139, 108, 255, 0.24), rgba(139, 108, 255, 0.1));
    box-shadow:
      inset 0 0 0 1px rgba(139, 108, 255, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    transition: background var(--duration-quick) var(--ease-smooth-out);
  }
  .chip:hover,
  .chip.open {
    background: linear-gradient(180deg, rgba(139, 108, 255, 0.36), rgba(139, 108, 255, 0.16));
  }
  .live .chip {
    background: linear-gradient(180deg, rgba(61, 242, 176, 0.2), rgba(61, 242, 176, 0.07));
    box-shadow: inset 0 0 0 1px rgba(61, 242, 176, 0.35);
  }
  .punct {
    margin: 0 4px 0 -1px;
  }

  /* Popover panels: labelled like OP Auto Clicker */
  .sentence :global(.pop) {
    font-size: 13.5px;
    font-weight: 450;
    letter-spacing: -0.006em;
    line-height: 1.4;
    color: var(--text);
    text-align: left;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin-bottom: 14px;
  }
  .field:last-child {
    margin-bottom: 0;
  }
  .label {
    display: block;
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 4px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .tile {
    height: 34px;
    border-radius: 9px;
    font-weight: 550;
    font-size: 12.5px;
    background: var(--glass);
    border: 1px solid var(--stroke);
  }
  .tile:hover {
    background: var(--glass-hover);
  }
  .tile.on {
    background: rgba(139, 108, 255, 0.2);
    border-color: rgba(139, 108, 255, 0.6);
  }
  .tile.wide {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .tile.listening {
    color: var(--violet-2);
    border-color: rgba(139, 108, 255, 0.7);
  }
  kbd {
    font-family: inherit;
    font-weight: 650;
    font-size: 12px;
    padding: 1px 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.14);
  }
  .hms {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .hms label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .hms input {
    width: 100%;
    text-align: right;
    font-weight: 650;
    font-size: 15px;
    padding: 7px 8px;
  }
  .hms small {
    font-size: 11px;
    color: var(--text-3);
    padding-left: 2px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 12px 0;
    font-size: 13px;
    color: var(--text-2);
  }
  input.small {
    width: 64px;
    padding: 4px 8px;
    text-align: right;
  }
  input.small:disabled {
    opacity: 0.4;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .preset {
    font-size: 12px;
    font-weight: 550;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--glass);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .preset:hover {
    color: var(--text);
  }
  .preset.on {
    color: var(--text);
    border-color: rgba(139, 108, 255, 0.6);
    background: rgba(139, 108, 255, 0.18);
  }
  .note {
    margin-top: 10px;
    font-size: 12px;
    color: var(--text-3);
  }
  .option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 9px 10px;
    border-radius: 11px;
  }
  .option {
    cursor: pointer;
  }
  .option:hover {
    background: var(--glass-hover);
  }
  .option b {
    display: block;
    font-weight: 600;
  }
  .option small {
    color: var(--text-3);
    font-size: 12px;
  }
  .option .inline {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-weight: 550;
  }
  .radio {
    flex: none;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    border: 1.5px solid var(--stroke-strong);
    transition: all var(--duration-fast) var(--ease-bounce);
  }
  .option.on .radio {
    border: 5px solid var(--violet);
    box-shadow: 0 0 12px rgba(139, 108, 255, 0.7);
  }
  .coords {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 4px 2px 38px;
  }
  .coords label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-3);
  }
  .coords input {
    width: 64px;
    padding: 5px 8px;
  }
  .btn.small {
    height: 30px;
    padding: 0 12px;
    font-size: 12.5px;
    margin-left: auto;
  }
  .btn.picking {
    color: var(--violet-2);
  }

  .hotkey-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 10px;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--stroke);
  }
  .muted {
    color: var(--text-3);
    font-size: 12.5px;
  }
  .spacer {
    flex: 1;
  }
  .warn {
    font-size: 12.5px;
    color: var(--amber);
    padding: 8px 12px;
    border-radius: 12px;
    background: rgba(255, 191, 92, 0.08);
    border: 1px solid rgba(255, 191, 92, 0.3);
  }
  .tune {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-top: 10px;
  }
  .val {
    min-width: 46px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text-2);
    font-size: 13px;
  }
  .tune-hint {
    font-size: 12px;
    color: var(--text-3);
    margin-top: 4px;
  }
</style>
