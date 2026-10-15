<script lang="ts">
  import { onDestroy } from "svelte";
  import { api } from "$lib/api";
  import { activity } from "$lib/activity.svelte";

  let { hotkey }: { hotkey: string | null } = $props();

  let now = $state(Date.now());
  // Ticks once a second, only while this bar exists (i.e. while recording).
  const t = setInterval(() => (now = Date.now()), 1000);
  onDestroy(() => clearInterval(t));

  let elapsed = $derived(Math.max(0, Math.floor((now - activity.recordingSince) / 1000)));
  let clock = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`);
</script>

<div class="bar" role="status">
  <span class="dot"></span>
  <b>Recording</b>
  <span class="clock">{clock}</span>
  <span class="hint">
    Go do your thing. Clicks inside hesytools are ignored.
    {#if hotkey}Press <kbd>{hotkey}</kbd> to stop.{/if}
  </span>
  <button class="stop" onclick={() => api.recordingStop()}>Stop</button>
</div>

<style>
  .bar {
    position: fixed;
    top: 60px;
    left: 50%;
    translate: -50% 0;
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 16px;
    border-radius: 999px;
    background: #1f1219;
    border: 1px solid rgba(255, 92, 112, 0.45);
    box-shadow:
      0 16px 40px -12px rgba(0, 0, 0, 0.7),
      0 0 30px -10px rgba(255, 92, 112, 0.6);
    animation: drop 350ms var(--ease-smooth-out) both;
    font-size: 13px;
    white-space: nowrap;
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
    font-variant-numeric: tabular-nums;
    color: #ffb3bd;
    font-weight: 650;
  }
  .hint {
    color: var(--text-3);
  }
  kbd {
    font-family: inherit;
    font-size: 11px;
    font-weight: 650;
    color: var(--text-2);
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--stroke);
  }
  .stop {
    font-weight: 650;
    padding: 6px 14px;
    border-radius: 999px;
    background: var(--red);
    color: white;
  }
  .stop:hover {
    filter: brightness(1.1);
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-10px) scale(0.97);
    }
  }
</style>
