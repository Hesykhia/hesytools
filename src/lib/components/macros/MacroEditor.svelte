<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, errorText, type Macro, type Step } from "$lib/api";
  import { activity, removeMacro, upsertMacro } from "$lib/activity.svelte";
  import { formatMs, durationMs, actionCount } from "$lib/steps";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "../HotkeyButton.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";
  import StepList from "./StepList.svelte";
  import ClickMap from "./ClickMap.svelte";

  let { source, onback }: { source: Macro; onback: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let m = $state<Macro>($state.snapshot(source));

  // ── Autosave (debounced) ────────────────────────────────────────────────
  let lastSaved = JSON.stringify(m);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saving = $state(false);
  $effect(() => {
    const snap = JSON.stringify(m);
    if (snap === lastSaved) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => save(snap), 400);
  });
  /** Returns false (after showing why) if the backend rejected it. */
  async function save(snap = JSON.stringify(m)): Promise<boolean> {
    saving = true;
    try {
      const saved = await api.saveMacro($state.snapshot(m));
      lastSaved = snap;
      upsertMacro(saved);
      return true;
    } catch (e) {
      toast(errorText(e), "error");
      return false;
    } finally {
      saving = false;
    }
  }
  onDestroy(() => {
    // Flush a pending edit when leaving the editor.
    if (JSON.stringify(m) !== lastSaved) {
      clearTimeout(saveTimer);
      api.saveMacro($state.snapshot(m)).then(upsertMacro).catch(() => {});
    }
    clearInterval(countdownTimer);
  });

  // ── Playback ────────────────────────────────────────────────────────────
  let playing = $derived(activity.playingId === m.id);
  let countdown = $state(0);
  let countdownTimer: ReturnType<typeof setInterval> | undefined;

  // Same 3s grace as the Quick Clicker so you can switch to the game first.
  async function playStop() {
    if (countdown > 0) {
      clearInterval(countdownTimer);
      countdown = 0;
      return;
    }
    if (playing) return api.stopMacro();
    if (JSON.stringify(m) !== lastSaved) {
      clearTimeout(saveTimer);
      await save();
    }
    countdown = 3;
    countdownTimer = setInterval(async () => {
      if (countdown <= 1) {
        clearInterval(countdownTimer);
        countdown = 0;
        await api.playMacro(m.id).catch((e) => toast(errorText(e), "error"));
      } else countdown -= 1;
    }, 1000);
  }

  async function setHotkey(v: string | null) {
    const prev = m.hotkey;
    m.hotkey = v;
    clearTimeout(saveTimer);
    // e.g. the key is already used elsewhere: put the old one back.
    if (!(await save())) m.hotkey = prev;
  }

  async function del() {
    if (playing) await api.stopMacro();
    await api.deleteMacro(m.id);
    removeMacro(m.id);
    lastSaved = JSON.stringify(m); // nothing to flush
    toast(`Deleted “${m.name}”`);
    onback();
  }

  const SPEEDS = [0.25, 0.5, 1, 1.5, 2, 4];
  let total = $derived(durationMs(m.steps) / (m.speed || 1));
  const placedClick = (steps: Step[]): boolean =>
    steps.some((s) => (s.type === "click" && !!s.at) || (s.type === "repeat" && placedClick(s.steps)));
  let hasClicks = $derived(placedClick(m.steps));
  let repeatText = $derived(m.repeat === 0 ? "until I stop it" : m.repeat === 1 ? "once" : `${m.repeat} times`);
</script>

<div class="editor">
  <div class="top">
    <button class="back" onclick={onback} aria-label="Back to macros">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 18l-6-6 6-6" /></svg>
      Macros
    </button>
    <span class="saved">{saving ? "Saving…" : "Saved"}</span>
  </div>

  <section class="head glass" class:live={playing}>
    <div class="title-col">
      <input class="name" bind:value={m.name} spellcheck="false" aria-label="Macro name" />
      <p class="meta">
        {actionCount(m.steps)} actions · {formatMs(total)} per run
        {#if playing}<span class="now">· playing{m.repeat !== 1 ? ` run ${activity.iteration + 1}` : ""}</span>{/if}
      </p>

      <p class="sentence">
        Play
        <Popover width={250}>
          {#snippet trigger({ toggle, open })}
            <button class="chip" class:open onclick={toggle}>{repeatText}</button>
          {/snippet}
          {#snippet children({ close })}
            <button class="menu-item" class:on={m.repeat === 1} onclick={() => ((m.repeat = 1), close())}>Once</button>
            <div class="menu-item times" class:on={m.repeat > 1}>
              <InlineNumber value={m.repeat > 1 ? m.repeat : 3} min={2} onchange={(v) => (m.repeat = Math.max(2, Math.round(v)))} />
              times
            </div>
            <button class="menu-item" class:on={m.repeat === 0} onclick={() => ((m.repeat = 0), close())}>Until I stop it</button>
          {/snippet}
        </Popover>
        at
        <Popover width={240}>
          {#snippet trigger({ toggle, open })}
            <button class="chip" class:open onclick={toggle}>{m.speed}× speed</button>
          {/snippet}
          {#snippet children({ close })}
            <div class="speeds">
              {#each SPEEDS as sp}
                <button class="speed" class:on={m.speed === sp} onclick={() => ((m.speed = sp), close())}>{sp}×</button>
              {/each}
            </div>
          {/snippet}
        </Popover>
      </p>

      <div class="hotkey-row">
        <span>Press</span>
        <HotkeyButton value={m.hotkey} onchange={setHotkey} placeholder="Set a hotkey" />
        <span>to start or stop this macro from anywhere</span>
      </div>
    </div>

    <button class="play" class:playing class:counting={countdown > 0} onclick={playStop}>
      {#if countdown > 0}
        <span class="cd">{countdown}</span>
        <small>click to cancel</small>
      {:else if playing}
        <svg width="22" height="22" viewBox="0 0 24 24"><rect x="6" y="6" width="12" height="12" rx="2.5" fill="currentColor" /></svg>
        <small>Stop</small>
      {:else}
        <svg width="24" height="24" viewBox="0 0 24 24"><path d="M8 5.5v13a1 1 0 0 0 1.5.87l11-6.5a1 1 0 0 0 0-1.74l-11-6.5A1 1 0 0 0 8 5.5z" fill="currentColor" /></svg>
        <small>Play</small>
      {/if}
    </button>
  </section>

  {#if hasClicks}
    <section class="map glass">
      <div class="map-head">
        <span class="eyebrow">Where it clicks</span>
        <span class="hint">Numbers match the steps below{playing ? "; the pulsing dot is playing now" : ""}.</span>
      </div>
      <div class="map-box">
        <ClickMap steps={m.steps} active={playing ? activity.step : -1} numbered />
      </div>
    </section>
  {/if}

  <section class="steps">
    {#if m.steps.length === 0}
      <p class="empty">No steps yet. Add one below, or record a new macro.</p>
    {/if}
    <StepList steps={m.steps} active={playing ? activity.step : -1} />
  </section>

  <div class="danger">
    <Popover width={250} align="center">
      {#snippet trigger({ toggle })}
        <button class="del" onclick={toggle}>Delete macro</button>
      {/snippet}
      {#snippet children({ close })}
        <p class="confirm">Delete “{m.name}”? This can't be undone.</p>
        <div class="confirm-row">
          <button class="btn" onclick={close}>Keep it</button>
          <button class="btn danger-btn" onclick={del}>Delete</button>
        </div>
      {/snippet}
    </Popover>
  </div>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-2);
    font-weight: 600;
    padding: 6px 10px 6px 4px;
    border-radius: 10px;
    transition: all var(--duration-quick) var(--ease-smooth-out);
  }
  .back:hover {
    color: var(--text);
    background: var(--glass);
  }
  .back svg {
    transition: transform var(--duration-fast) var(--ease-smooth-out);
  }
  .back:hover svg {
    transform: translateX(-2px);
  }
  .saved {
    font-size: 12px;
    color: var(--text-3);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 24px 28px;
    transition:
      border-color 600ms var(--ease-in-out),
      box-shadow 600ms var(--ease-in-out);
  }
  .head.live {
    border-color: rgba(61, 242, 176, 0.3);
    box-shadow: 0 0 50px -20px rgba(61, 242, 176, 0.45);
  }
  .title-col {
    flex: 1;
    min-width: 0;
  }
  .name {
    width: 100%;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.03em;
    padding: 2px 8px;
    margin-left: -9px;
    background: transparent !important;
    border-color: transparent !important;
    box-shadow: none !important;
    border-radius: 10px;
  }
  .name:hover {
    background: var(--glass) !important;
  }
  .name:focus {
    background: rgba(0, 0, 0, 0.25) !important;
  }
  .meta {
    color: var(--text-3);
    font-size: 13px;
    margin: 2px 0 16px;
  }
  .now {
    color: var(--mint);
  }
  .sentence {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    font-size: 16px;
    font-weight: 550;
    color: var(--text-2);
  }
  .sentence :global(.num) {
    font-size: 15px;
  }
  .chip {
    font-weight: 650;
    padding: 3px 11px;
    border-radius: 9px;
    color: var(--text);
    background: rgba(139, 108, 255, 0.18);
    box-shadow: inset 0 0 0 1px rgba(139, 108, 255, 0.4);
  }
  .chip:hover,
  .chip.open {
    background: rgba(139, 108, 255, 0.3);
  }
  .hotkey-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
    padding: 6px 14px 6px 12px;
    border-radius: 14px;
    font-size: 13.5px;
    color: var(--text-2);
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--stroke);
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border-radius: 9px;
    font-size: 14px;
    font-weight: 550;
    color: var(--text);
  }
  button.menu-item:hover {
    background: var(--glass-hover);
  }
  .menu-item.on {
    color: var(--violet-2);
  }
  .menu-item.times {
    color: var(--text-2);
  }
  .speeds {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .speed {
    height: 34px;
    border-radius: 9px;
    font-weight: 650;
    background: var(--glass);
    border: 1px solid var(--stroke);
  }
  .speed.on {
    background: rgba(139, 108, 255, 0.22);
    border-color: rgba(139, 108, 255, 0.6);
  }

  .play {
    flex: none;
    width: 92px;
    height: 92px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    color: white;
    background:
      radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.25), transparent 60%),
      linear-gradient(180deg, #9a7fff, #6c4cf0);
    box-shadow:
      0 12px 32px -8px rgba(139, 108, 255, 0.8),
      inset 0 1px 0 rgba(255, 255, 255, 0.3);
    transition:
      transform 350ms var(--ease-bounce),
      box-shadow 300ms var(--ease-smooth-out),
      background 400ms var(--ease-in-out);
  }
  .play:hover {
    transform: scale(1.05);
  }
  .play:active {
    transform: scale(0.96);
  }
  .play svg {
    margin-left: 2px;
  }
  .play small {
    font-size: 11px;
    font-weight: 650;
    opacity: 0.85;
  }
  .play.playing {
    background:
      radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.3), transparent 60%),
      linear-gradient(180deg, #3df2b0, #12b886);
    color: #052e22;
    box-shadow: 0 12px 32px -8px rgba(61, 242, 176, 0.7);
  }
  .play.playing svg {
    margin: 0;
  }
  .play.counting {
    background: linear-gradient(180deg, #ffcf7a, #f59f2a);
    color: #3b2300;
    box-shadow: 0 12px 32px -8px rgba(255, 191, 92, 0.7);
  }
  .cd {
    font-size: 30px;
    font-weight: 800;
    line-height: 1;
  }

  .steps {
    padding: 0 2px;
  }
  .map {
    padding: 14px 16px;
  }
  .map-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .map-head .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .map-box {
    height: 150px;
  }
  .empty {
    color: var(--text-3);
    padding: 6px 4px 4px;
  }
  .danger {
    display: flex;
    justify-content: center;
    padding: 8px 0 4px;
  }
  .del {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-3);
    padding: 6px 12px;
    border-radius: 99px;
  }
  .del:hover {
    color: var(--red);
    background: rgba(255, 92, 112, 0.08);
  }
  .confirm {
    font-size: 13px;
    color: var(--text-2);
    margin-bottom: 12px;
  }
  .confirm-row {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .danger-btn {
    background: var(--red) !important;
    border-color: transparent !important;
    color: white;
  }
</style>
