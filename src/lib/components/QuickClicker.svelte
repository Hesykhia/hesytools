<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    api,
    errorText,
    MOUSE_BUTTONS,
    onClickerState,
    type ClickerConfig,
    type HotkeyMode,
    type MouseButton,
    type Settings,
  } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "./HotkeyButton.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

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
    if (out.stopAfter.kind === "clicks")
      out.stopAfter = { kind: "clicks", count: Math.round(num(out.stopAfter.count, 1, 1)) };
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

  // ── Interval, editable as ms / seconds / clicks-per-second ───────────────
  type Unit = "ms" | "s" | "cps";
  let unit = $state<Unit>("ms");
  const round = (n: number) => Math.round(n * 1000) / 1000;
  let shownInterval = $derived(
    unit === "ms" ? round(cfg.intervalMs) : unit === "s" ? round(cfg.intervalMs / 1000) : round(1000 / cfg.intervalMs),
  );
  function setInterval_(raw: string) {
    const v = parseFloat(raw);
    if (!(v > 0)) return;
    const ms = unit === "ms" ? v : unit === "s" ? v * 1000 : 1000 / v;
    cfg.intervalMs = Math.max(1, Math.min(ms, 3_600_000));
  }

  // ── Running state + live CPS ─────────────────────────────────────────────
  let running = $state(false);
  let clicks = $state(0);
  let blocked = $state(0);
  let cps = $state(0);
  let countdown = $state(0);
  let countdownTimer: ReturnType<typeof setInterval> | undefined;
  let pollTimer: ReturnType<typeof setInterval> | undefined;
  let last = { clicks: 0, at: 0 };

  async function poll() {
    const s = await api.clickerStatus();
    const now = performance.now();
    if (s.clicks < last.clicks) last = { clicks: 0, at: now }; // run restarted
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
    const s = await api.clickerStatus();
    setRunning(s.running);
  });
  onDestroy(() => {
    unlisten?.();
    clearInterval(pollTimer);
    clearInterval(countdownTimer);
    clearTimeout(saveTimer);
  });

  // Starting from the button waits 3s so you can move to the target first.
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
      countdown -= 1;
      if (countdown <= 0) {
        clearInterval(countdownTimer);
        await api.clickerToggle().catch((e) => toast(errorText(e), "error"));
      }
    }, 1000);
  }

  // ── Action / position helpers ────────────────────────────────────────────
  function setActionKind(kind: "mouse" | "key") {
    if (kind === cfg.action.kind) return;
    cfg.action = kind === "mouse" ? { kind: "mouse", button: "LeftMouse" } : { kind: "key", key: "E" };
  }

  function setActionFromCapture(combo: string | null) {
    if (!combo) return;
    const trigger = combo.split("+").pop()!;
    const mouse = MOUSE_BUTTONS.find((b) => b.value === trigger);
    cfg.action = mouse ? { kind: "mouse", button: mouse.value } : { kind: "key", key: trigger };
  }

  let picking = $state(false);
  async function pick() {
    if (picking) {
      await api.cancelPick();
      return;
    }
    picking = true;
    try {
      const p = await api.pickPoint();
      if (p) cfg.position = { kind: "fixed", x: p[0], y: p[1] };
    } finally {
      picking = false;
    }
  }

  function setStopKind(kind: "never" | "clicks" | "seconds") {
    if (kind === cfg.stopAfter.kind) return;
    cfg.stopAfter =
      kind === "never" ? { kind } : kind === "clicks" ? { kind, count: 100 } : { kind, secs: 60 };
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

  let hotkeyLabel = $derived(settings.clickerHotkey ?? "no hotkey");
</script>

<section class="card clicker" class:running>
  <header>
    <div>
      <h2>Quick Clicker</h2>
      <p class="muted sub">Repeatedly click a mouse button or press a key.</p>
    </div>
    <div class="status">
      {#if running}
        <span class="pill live"><span class="dot"></span>{cps.toFixed(1)} CPS · {clicks.toLocaleString()} clicks</span>
      {:else if clicks > 0}
        <span class="pill">Stopped · {clicks.toLocaleString()} clicks</span>
      {:else}
        <span class="pill">Stopped</span>
      {/if}
      <button class="go" class:primary={!running && !countdown} class:danger={running} onclick={startStop}>
        {#if countdown > 0}
          Starting in {countdown}… (click to cancel)
        {:else if running}
          Stop
        {:else}
          Start
        {/if}
      </button>
    </div>
  </header>

  {#if blocked > 0}
    <div class="warn">
      Windows blocked {blocked.toLocaleString()} clicks. The window you're clicking is probably running as administrator.
      Restart hesytools as admin from Settings.
    </div>
  {/if}

  <div class="grid">
    <span class="label">Click</span>
    <div class="row">
      <div class="seg">
        <button class:on={cfg.action.kind === "mouse"} onclick={() => setActionKind("mouse")}>Mouse</button>
        <button class:on={cfg.action.kind === "key"} onclick={() => setActionKind("key")}>Keyboard</button>
      </div>
      {#if cfg.action.kind === "mouse"}
        <select
          value={cfg.action.button}
          onchange={(e) => (cfg.action = { kind: "mouse", button: e.currentTarget.value as MouseButton })}
        >
          {#each MOUSE_BUTTONS as b}
            <option value={b.value}>{b.label}</option>
          {/each}
        </select>
      {:else}
        <HotkeyButton value={cfg.action.key} clearable={false} onchange={setActionFromCapture} />
        <span class="muted hint">Click, then press the key to spam</span>
      {/if}
    </div>

    <span class="label">Every</span>
    <div class="row">
      <input type="number" min="0" step="any" value={shownInterval} onchange={(e) => setInterval_(e.currentTarget.value)} />
      <div class="seg">
        <button class:on={unit === "ms"} onclick={() => (unit = "ms")}>ms</button>
        <button class:on={unit === "s"} onclick={() => (unit = "s")}>sec</button>
        <button class:on={unit === "cps"} onclick={() => (unit = "cps")}>per sec</button>
      </div>
      <span class="muted hint">
        {unit === "cps" ? `= every ${round(cfg.intervalMs)} ms` : `= ${round(1000 / cfg.intervalMs)} per second`}
      </span>
    </div>

    <span class="label">Hold for</span>
    <div class="row">
      <input type="number" min="0" step="any" bind:value={cfg.holdMs} />
      <span class="muted">ms</span>
      <span class="label inline">Randomize ±</span>
      <input type="number" min="0" step="any" bind:value={cfg.jitterMs} />
      <span class="muted">ms</span>
    </div>

    <span class="label">Where</span>
    <div class="row">
      <div class="seg">
        <button class:on={cfg.position.kind === "current"} onclick={() => (cfg.position = { kind: "current" })}>
          Wherever the cursor is
        </button>
        <button
          class:on={cfg.position.kind === "fixed"}
          onclick={() => cfg.position.kind !== "fixed" && pick()}
          disabled={cfg.action.kind === "key"}
        >
          Fixed spot
        </button>
      </div>
      {#if cfg.position.kind === "fixed" && cfg.action.kind === "mouse"}
        <span class="coords">X <input type="number" bind:value={cfg.position.x} /> Y <input type="number" bind:value={cfg.position.y} /></span>
        <button onclick={pick} class:picking>{picking ? "Click anywhere… (Esc cancels)" : "Pick"}</button>
      {:else if picking}
        <span class="picking-hint">Click anywhere on screen… (Esc cancels)</span>
      {/if}
    </div>

    <span class="label">Stop after</span>
    <div class="row">
      <div class="seg">
        <button class:on={cfg.stopAfter.kind === "never"} onclick={() => setStopKind("never")}>Never</button>
        <button class:on={cfg.stopAfter.kind === "clicks"} onclick={() => setStopKind("clicks")}>Clicks</button>
        <button class:on={cfg.stopAfter.kind === "seconds"} onclick={() => setStopKind("seconds")}>Time</button>
      </div>
      {#if cfg.stopAfter.kind === "clicks"}
        <input type="number" min="1" bind:value={cfg.stopAfter.count} /> <span class="muted">clicks</span>
      {:else if cfg.stopAfter.kind === "seconds"}
        <input type="number" min="0.1" step="any" bind:value={cfg.stopAfter.secs} /> <span class="muted">seconds</span>
      {/if}
    </div>

    <span class="label">Hotkey</span>
    <div class="row">
      <HotkeyButton value={settings.clickerHotkey} onchange={setHotkey} />
      <div class="seg">
        <button class:on={settings.clickerMode === "toggle"} onclick={() => setMode("toggle")}>Press to toggle</button>
        <button class:on={settings.clickerMode === "hold"} onclick={() => setMode("hold")}>Hold to click</button>
      </div>
    </div>
  </div>

  <p class="muted foot">
    {#if settings.clickerMode === "hold"}
      Hold <b>{hotkeyLabel}</b> to click. Works while a game is focused.
    {:else}
      Press <b>{hotkeyLabel}</b> to start/stop. Works while a game is focused.
    {/if}
    Games usually need a hold of 10–20 ms to register a key press.
  </p>
</section>

<style>
  .clicker {
    transition: border-color 0.2s;
  }
  .clicker.running {
    border-color: rgba(62, 207, 142, 0.45);
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 16px;
  }
  .sub {
    margin: 4px 0 0;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .go {
    min-width: 96px;
    padding: 8px 16px;
    font-weight: 600;
  }
  .pill {
    font-size: 12px;
    color: var(--muted);
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 4px 10px;
    font-variant-numeric: tabular-nums;
  }
  .pill.live {
    color: var(--good);
    border-color: rgba(62, 207, 142, 0.35);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--good);
    animation: blink 1s infinite;
  }
  .grid {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 12px 14px;
    align-items: center;
  }
  .label {
    color: var(--muted);
    font-size: 13px;
  }
  .label.inline {
    margin-left: 14px;
  }
  .hint {
    font-size: 12px;
  }
  .coords {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .coords input {
    width: 76px;
  }
  .picking,
  .picking-hint {
    color: var(--accent-2);
  }
  .warn {
    background: rgba(245, 165, 36, 0.1);
    border: 1px solid rgba(245, 165, 36, 0.35);
    color: var(--warn);
    border-radius: 8px;
    padding: 8px 12px;
    margin-bottom: 14px;
    font-size: 13px;
  }
  .foot {
    font-size: 12px;
    margin: 16px 0 0;
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }
</style>
