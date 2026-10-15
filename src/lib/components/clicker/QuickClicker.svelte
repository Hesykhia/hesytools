<script lang="ts">
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

  // ── Rate, shown in whichever unit reads best ─────────────────────────────
  type Unit = "cps" | "ms" | "s";
  const round = (n: number, d = 2) => Math.round(n * 10 ** d) / 10 ** d;
  function guessUnit(ms: number): Unit {
    if (ms >= 1000) return "s";
    return Number.isInteger(round(1000 / ms, 4)) ? "cps" : "ms";
  }
  // svelte-ignore state_referenced_locally
  let unit = $state<Unit>((localStorage.getItem("clicker-unit") as Unit) || guessUnit(cfg.intervalMs));
  $effect(() => localStorage.setItem("clicker-unit", unit));

  let shownRate = $derived(
    unit === "ms" ? round(cfg.intervalMs) : unit === "s" ? round(cfg.intervalMs / 1000) : round(1000 / cfg.intervalMs),
  );
  function setRate(v: number) {
    if (!(v > 0)) return;
    const ms = unit === "ms" ? v : unit === "s" ? v * 1000 : 1000 / v;
    cfg.intervalMs = Math.max(1, Math.min(ms, 3_600_000));
  }
  const PRESETS: { label: string; ms: number; unit: Unit }[] = [
    { label: "Max (1 ms)", ms: 1, unit: "ms" },
    { label: "50 / sec", ms: 20, unit: "cps" },
    { label: "20 / sec", ms: 50, unit: "cps" },
    { label: "10 / sec", ms: 100, unit: "cps" },
    { label: "Every second", ms: 1000, unit: "s" },
  ];

  // ── The sentence ──────────────────────────────────────────────────────────
  const buttonLabel = (b: string) =>
    ({ LeftMouse: "Left click", RightMouse: "Right click", MiddleMouse: "Middle click", Mouse4: "Click Mouse 4", Mouse5: "Click Mouse 5" })[b] ?? b;

  let actionText = $derived(cfg.action.kind === "mouse" ? buttonLabel(cfg.action.button) : `Press ${cfg.action.key}`);
  let rateText = $derived.by(() => {
    const v = shownRate;
    if (unit === "cps") return v === 1 ? "once a second" : `${v} times a second`;
    if (unit === "s") return v === 1 ? "every second" : `every ${v} seconds`;
    return `every ${v} ms`;
  });
  let whereText = $derived(
    cfg.position.kind === "current" ? "wherever my cursor is" : `at ${cfg.position.x}, ${cfg.position.y}`,
  );
  let stopText = $derived(
    cfg.stopAfter.kind === "never"
      ? "until I stop it"
      : cfg.stopAfter.kind === "clicks"
        ? `and stop after ${cfg.stopAfter.count} ${cfg.action.kind === "key" ? "presses" : "clicks"}`
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

  /** Switching to a fixed spot goes straight into picking it on screen. */
  function chooseFixed() {
    if (cfg.position.kind === "fixed") return;
    cfg.position = { kind: "fixed", x: 0, y: 0 };
    pick();
  }

  function setStopKind(kind: "never" | "clicks" | "seconds") {
    cfg.stopAfter = kind === "never" ? { kind } : kind === "clicks" ? { kind, count: 100 } : { kind, secs: 60 };
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

<section class="hero glass" class:live={running}>
  <div class="orb-col">
    <ClickerOrb {phase} {cps} {countdown} onclick={startStop} />
    <div class="tally" class:show={clicks > 0}>
      {clicks.toLocaleString()} {cfg.action.kind === "key" ? "presses" : "clicks"} this run
    </div>
  </div>

  <div class="main">
    <span class="eyebrow">Quick Clicker</span>

    <p class="sentence">
      <Popover width={330}>
        {#snippet trigger({ toggle, open })}
          <button class="chip" class:open onclick={toggle}>{actionText}</button>
        {/snippet}
        {#snippet children({ close })}
          <div class="eyebrow pad">Mouse button</div>
          <div class="tiles">
            {#each MOUSE_BUTTONS as b}
              <button
                class="tile"
                class:on={cfg.action.kind === "mouse" && cfg.action.button === b.value}
                onclick={() => {
                  cfg.action = { kind: "mouse", button: b.value };
                  close();
                }}
              >
                {b.label.replace(/ \(.*\)/, "")}
              </button>
            {/each}
          </div>
          <div class="eyebrow pad top">Or a keyboard key</div>
          <button class="tile wide" class:on={cfg.action.kind === "key"} class:listening={capturingKey} onclick={captureKey}>
            {#if capturingKey}
              Press any key…
            {:else if cfg.action.kind === "key"}
              Spamming <kbd>{cfg.action.key}</kbd>, click to change
            {:else}
              Choose a key…
            {/if}
          </button>
        {/snippet}
      </Popover>

      <Popover width={320}>
        {#snippet trigger({ toggle, open })}
          <button class="chip" class:open onclick={toggle}>{rateText}</button>{#if cfg.action.kind !== "mouse"}<span
              class="punct">,</span
            >{/if}
        {/snippet}
        {#snippet children()}
          <div class="center">
            <Tabs
              size="s"
              value={unit}
              onchange={(u) => (unit = u)}
              options={[
                { value: "cps", label: "Per second" },
                { value: "ms", label: "Milliseconds" },
                { value: "s", label: "Seconds" },
              ]}
            />
          </div>
          <div class="big-input">
            <input type="number" min="0" step="any" value={shownRate} oninput={(e) => setRate(parseFloat(e.currentTarget.value))} />
            <span>{unit === "cps" ? "per sec" : unit === "ms" ? "ms" : "sec"}</span>
          </div>
          <div class="presets">
            {#each PRESETS as p}
              <button
                class="preset"
                class:on={Math.abs(cfg.intervalMs - p.ms) < 0.001}
                onclick={() => {
                  unit = p.unit;
                  cfg.intervalMs = p.ms;
                }}>{p.label}</button
              >
            {/each}
          </div>
          <p class="note">
            {unit === "cps" ? `One click every ${round(cfg.intervalMs)} ms.` : `That's ${round(1000 / cfg.intervalMs)} clicks per second.`}
            Some games cap how fast they read input.
          </p>
        {/snippet}
      </Popover>

      {#if cfg.action.kind === "mouse"}
        <Popover width={330}>
          {#snippet trigger({ toggle, open })}
            <button class="chip" class:open onclick={toggle}>{whereText}</button><span class="punct">,</span>
          {/snippet}
          {#snippet children()}
            <button class="option" class:on={cfg.position.kind === "current"} onclick={() => (cfg.position = { kind: "current" })}>
              <span class="radio"></span>
              <span><b>Wherever my cursor is</b><small>Never moves your mouse</small></span>
            </button>
            <button
              class="option"
              class:on={cfg.position.kind === "fixed"}
              onclick={chooseFixed}
            >
              <span class="radio"></span>
              <span><b>A fixed spot</b><small>Jumps there before every click</small></span>
            </button>
            {#if cfg.position.kind === "fixed"}
              <div class="coords">
                <label>X <input type="number" bind:value={cfg.position.x} /></label>
                <label>Y <input type="number" bind:value={cfg.position.y} /></label>
                <button class="btn pickbtn" class:picking onclick={pick}>{picking ? "Click anywhere…" : "Pick on screen"}</button>
              </div>
              {#if picking}<p class="note">Click the spot you want. Esc cancels.</p>{/if}
            {/if}
          {/snippet}
        </Popover>{/if}

      <Popover width={300}>
        {#snippet trigger({ toggle, open })}
          <button class="chip" class:open onclick={toggle}>{stopText}</button><span class="punct">.</span>
        {/snippet}
        {#snippet children()}
          <div class="center">
            <Tabs
              size="s"
              value={cfg.stopAfter.kind}
              onchange={setStopKind}
              options={[
                { value: "never", label: "Never" },
                { value: "clicks", label: "After clicks" },
                { value: "seconds", label: "After time" },
              ]}
            />
          </div>
          {#if cfg.stopAfter.kind === "clicks"}
            <div class="big-input">
              <input type="number" min="1" bind:value={cfg.stopAfter.count} />
              <span>{cfg.action.kind === "key" ? "presses" : "clicks"}</span>
            </div>
          {:else if cfg.stopAfter.kind === "seconds"}
            <div class="big-input">
              <input type="number" min="0.1" step="any" bind:value={cfg.stopAfter.secs} />
              <span>seconds</span>
            </div>
          {:else}
            <p class="note">Keeps going until you press the hotkey, the panic key, or the orb.</p>
          {/if}
        {/snippet}
      </Popover>
    </p>

    <div class="hotkey-row">
      <span class="muted">{settings.clickerMode === "hold" ? "Hold" : "Press"}</span>
      <HotkeyButton value={settings.clickerHotkey} onchange={setHotkey} />
      <span class="muted">{settings.clickerMode === "hold" ? "to click" : "to start or stop, even in games"}</span>
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
        Windows blocked {blocked.toLocaleString()} clicks because that window is running as admin. Restart hesytools as admin
        from Settings.
      </div>
    {/if}

    <Disclosure title="Fine-tune">
      <div class="tune">
        <div class="tune-row">
          <div>
            <b>Hold each click</b>
            <small>Games often need 10–20 ms to notice a press</small>
          </div>
          <Slider value={cfg.holdMs} max={100} onchange={(v) => (cfg.holdMs = v)} />
          <span class="val">{Math.round(cfg.holdMs)} ms</span>
        </div>
        <div class="tune-row">
          <div>
            <b>Randomize timing</b>
            <small>Varies each gap so it looks less robotic</small>
          </div>
          <Slider value={cfg.jitterMs} max={Math.max(10, Math.round(cfg.intervalMs / 2))} onchange={(v) => (cfg.jitterMs = v)} />
          <span class="val">± {Math.round(cfg.jitterMs)} ms</span>
        </div>
      </div>
    </Disclosure>
  </div>
</section>

<style>
  .hero {
    position: relative;
    display: grid;
    grid-template-columns: 250px 1fr;
    gap: 28px;
    padding: 30px 34px 22px 24px;
    transition:
      border-color 800ms var(--ease-in-out),
      box-shadow 800ms var(--ease-in-out);
  }
  .hero.live {
    border-color: rgba(61, 242, 176, 0.25);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.06),
      0 0 60px -20px rgba(61, 242, 176, 0.35);
  }
  .orb-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }
  .tally {
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
    opacity: 0;
    transform: translateY(4px);
    transition:
      opacity var(--duration-fast) var(--ease-smooth-out),
      transform var(--duration-fast) var(--ease-smooth-out);
  }
  .tally.show {
    opacity: 1;
    transform: none;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding-top: 6px;
    min-width: 0;
  }

  /* The editable sentence */
  .sentence {
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.025em;
    line-height: 1.75;
    color: var(--text-2);
  }
  .chip {
    font-size: inherit;
    font-weight: 650;
    padding: 0 12px;
    margin: 0 2px;
    border-radius: 12px;
    color: var(--text);
    background: linear-gradient(180deg, rgba(139, 108, 255, 0.24), rgba(139, 108, 255, 0.1));
    box-shadow:
      inset 0 0 0 1px rgba(139, 108, 255, 0.4),
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
    transition:
      background var(--duration-quick) var(--ease-smooth-out),
      box-shadow var(--duration-fast) var(--ease-smooth-out),
      transform var(--duration-quick) var(--ease-smooth-out);
  }
  .chip:hover {
    background: linear-gradient(180deg, rgba(139, 108, 255, 0.34), rgba(139, 108, 255, 0.16));
  }
  .chip:active {
    transform: scale(0.98);
  }
  .chip.open {
    box-shadow:
      inset 0 0 0 1px rgba(180, 156, 255, 0.8),
      0 0 0 4px rgba(139, 108, 255, 0.18),
      0 0 24px -4px rgba(139, 108, 255, 0.7);
  }
  /* Punctuation lives inside each popover's anchor so it hugs its chip. */
  .punct {
    margin: 0 6px 0 -2px;
  }
  .live .chip {
    background: linear-gradient(180deg, rgba(61, 242, 176, 0.2), rgba(61, 242, 176, 0.07));
    box-shadow: inset 0 0 0 1px rgba(61, 242, 176, 0.35);
  }

  /* Popover contents (popovers render inside the sentence, so reset type) */
  .sentence :global(.pop) {
    font-size: 14px;
    font-weight: 450;
    letter-spacing: -0.006em;
    line-height: 1.4;
    color: var(--text);
  }
  .pad {
    padding: 2px 4px 8px;
  }
  .pad.top {
    padding-top: 14px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .tile {
    height: 38px;
    border-radius: 10px;
    font-weight: 550;
    font-size: 13px;
    background: var(--glass);
    border: 1px solid var(--stroke);
    transition:
      background var(--duration-quick) var(--ease-smooth-out),
      border-color var(--duration-quick) var(--ease-smooth-out);
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
    animation: listen 1.4s ease-in-out infinite;
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
  .center {
    display: flex;
    justify-content: center;
  }
  .big-input {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 8px;
    margin: 16px 0 12px;
  }
  .big-input input {
    width: 150px;
    text-align: center;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.03em;
    padding: 6px 10px;
    border-radius: 14px;
  }
  .big-input span {
    color: var(--text-3);
    font-weight: 550;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    justify-content: center;
  }
  .preset {
    font-size: 12px;
    font-weight: 550;
    padding: 5px 10px;
    border-radius: 999px;
    background: var(--glass);
    border: 1px solid var(--stroke);
    color: var(--text-2);
    transition: all var(--duration-quick) var(--ease-smooth-out);
  }
  .preset:hover {
    color: var(--text);
    background: var(--glass-hover);
  }
  .preset.on {
    color: var(--text);
    border-color: rgba(139, 108, 255, 0.6);
    background: rgba(139, 108, 255, 0.18);
  }
  .note {
    margin-top: 12px;
    font-size: 12px;
    color: var(--text-3);
    text-align: center;
  }
  .option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 10px 12px;
    border-radius: 12px;
    transition: background var(--duration-quick) var(--ease-smooth-out);
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
  .radio {
    flex: none;
    width: 18px;
    height: 18px;
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
    padding: 10px 6px 2px;
  }
  .coords label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
    font-size: 12px;
    font-weight: 600;
  }
  .coords input {
    width: 70px;
    padding: 7px 9px;
  }
  .pickbtn {
    height: 32px;
    padding: 0 12px;
    font-size: 12.5px;
    margin-left: auto;
  }
  .pickbtn.picking {
    color: var(--violet-2);
    border-color: rgba(139, 108, 255, 0.6);
  }

  .hotkey-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding: 10px 12px 10px 16px;
    border-radius: 16px;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--stroke);
  }
  .muted {
    color: var(--text-2);
  }
  .spacer {
    flex: 1;
  }
  .warn {
    font-size: 13px;
    color: var(--amber);
    padding: 10px 14px;
    border-radius: 14px;
    background: rgba(255, 191, 92, 0.08);
    border: 1px solid rgba(255, 191, 92, 0.3);
  }
  .tune {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px 4px 4px;
  }
  .tune-row {
    display: grid;
    grid-template-columns: 220px 1fr 64px;
    align-items: center;
    gap: 16px;
  }
  .tune-row b {
    display: block;
    font-weight: 600;
    font-size: 13px;
  }
  .tune-row small {
    color: var(--text-3);
    font-size: 12px;
  }
  .val {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text-2);
    font-size: 13px;
  }
  @keyframes listen {
    50% {
      box-shadow: 0 0 0 4px rgba(139, 108, 255, 0.15);
    }
  }
</style>
