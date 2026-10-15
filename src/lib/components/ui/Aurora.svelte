<script lang="ts">
  // Ambient background glow. Two palettes crossfade so the whole window
  // tells you at a glance whether something is running.
  //
  // The blobs are soft radial gradients rather than `filter: blur()`:
  // animating huge blurred layers under translucent cards made WebView2
  // recomposite every frame, which flickered and cost GPU/CPU. It also
  // doesn't drift: an idle app should draw nothing at all.
  let { active = false }: { active?: boolean } = $props();
</script>

<div class="aurora" aria-hidden="true">
  <div class="layer idle" class:off={active}></div>
  <div class="layer live" class:off={!active}></div>
  <div class="vignette"></div>
</div>

<style>
  .aurora {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    background: var(--bg);
    contain: strict;
  }
  .layer {
    position: absolute;
    inset: 0;
    opacity: 1;
    transition: opacity 1.2s var(--ease-in-out);
  }
  .layer.off {
    opacity: 0;
  }
  .idle {
    background:
      radial-gradient(38% 30% at 30% 18%, rgba(139, 108, 255, 0.34), transparent 70%),
      radial-gradient(34% 28% at 72% 14%, rgba(91, 140, 255, 0.24), transparent 70%),
      radial-gradient(30% 26% at 55% 92%, rgba(255, 110, 199, 0.14), transparent 70%);
  }
  .live {
    background:
      radial-gradient(38% 30% at 30% 18%, rgba(61, 242, 176, 0.24), transparent 70%),
      radial-gradient(34% 28% at 72% 14%, rgba(56, 214, 255, 0.2), transparent 70%),
      radial-gradient(30% 26% at 55% 92%, rgba(61, 242, 176, 0.12), transparent 70%);
  }
  .vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(140% 100% at 50% 0%, transparent 55%, rgba(0, 0, 0, 0.45) 100%);
  }
</style>
