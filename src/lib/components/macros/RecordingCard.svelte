<script lang="ts">
  // Shown at the top of the Macros page while recording.
  import { activity } from "$lib/activity.svelte";
  import LiveRecording from "./LiveRecording.svelte";
</script>

<div class="live glass">
  <div class="live-head">
    <span class="dot"></span>
    <b>Recording live</b>
    <span class="hint">Every click and key shows up here as you go. Stop to open it in the editor.</span>
  </div>
  <LiveRecording steps={activity.liveSteps} />
</div>

<style>
  .live {
    padding: 18px 20px;
    border-color: rgba(255, 92, 112, 0.35);
    animation: rise 350ms var(--ease-smooth-out) both;
  }
  .live-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .live-head .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 10px var(--red);
  }
  .live-head .hint {
    font-size: 12.5px;
    color: var(--text-3);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
