<script lang="ts">
  // The always-on-top recording overlay (a separate small window). It only
  // listens; the main window owns saving the recording.
  import { onMount } from "svelte";
  import { api, onRecordingProgress, type Step } from "$lib/api";
  import LiveRecording from "$lib/components/macros/LiveRecording.svelte";

  let steps = $state<Step[]>([]);
  let hotkey = $state<string | null>(null);
  let started = Date.now();
  let now = $state(Date.now());
  let elapsed = $derived(Math.max(0, Math.floor((now - started) / 1000)));
  let clock = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`);

  onMount(() => {
    // The overlay window is transparent; only the card should be visible.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    api.getSettings().then((s) => (hotkey = s.recordHotkey));
    const t = setInterval(() => (now = Date.now()), 1000);
    const un = onRecordingProgress((s) => (steps = s));
    return () => {
      clearInterval(t);
      un.then((f) => f());
    };
  });
</script>

<div class="hud" data-tauri-drag-region>
  <header data-tauri-drag-region>
    <span class="dot"></span>
    <b data-tauri-drag-region>Recording</b>
    <span class="clock" data-tauri-drag-region>{clock}</span>
    <button class="stop" onclick={() => api.recordingStop()}>Stop{#if hotkey}<kbd>{hotkey}</kbd>{/if}</button>
  </header>
  <LiveRecording {steps} compact />
</div>

<style>
  .hud {
    margin: 6px;
    padding: 12px;
    border-radius: 20px;
    background: rgba(18, 12, 18, 0.94);
    border: 1px solid rgba(255, 92, 112, 0.4);
    box-shadow: 0 0 24px -8px rgba(255, 92, 112, 0.5);
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: calc(100vh - 12px);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 13px;
    padding: 0 2px;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 10px var(--red);
    animation: blink 1.2s ease-in-out infinite;
  }
  .clock {
    color: #ffb3bd;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .stop {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 650;
    font-size: 12.5px;
    padding: 5px 6px 5px 12px;
    border-radius: 999px;
    background: var(--red);
    color: white;
  }
  .stop kbd {
    font-family: inherit;
    font-size: 10.5px;
    padding: 1px 5px;
    border-radius: 5px;
    background: rgba(0, 0, 0, 0.25);
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
</style>
