<script lang="ts">
  // Ambient background glow. Two palettes crossfade so the whole window
  // tells you at a glance whether something is running.
  let { active = false }: { active?: boolean } = $props();
</script>

<div class="aurora" aria-hidden="true">
  <div class="layer idle" class:off={active}>
    <span class="blob a"></span>
    <span class="blob b"></span>
    <span class="blob c"></span>
  </div>
  <div class="layer live" class:off={!active}>
    <span class="blob a"></span>
    <span class="blob b"></span>
    <span class="blob c"></span>
  </div>
  <div class="grain"></div>
  <div class="vignette"></div>
</div>

<style>
  .aurora {
    position: fixed;
    inset: 0;
    z-index: 0;
    overflow: hidden;
    pointer-events: none;
    background: radial-gradient(120% 80% at 50% -10%, #12101f 0%, var(--bg) 60%);
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
  .blob {
    position: absolute;
    border-radius: 50%;
    filter: blur(90px);
    opacity: 0.55;
    will-change: transform;
  }
  .a {
    width: 520px;
    height: 420px;
    top: -220px;
    left: 8%;
    animation: drift-a 22s ease-in-out infinite alternate;
  }
  .b {
    width: 460px;
    height: 380px;
    top: -200px;
    right: 4%;
    animation: drift-b 26s ease-in-out infinite alternate;
  }
  .c {
    width: 380px;
    height: 300px;
    bottom: -260px;
    left: 35%;
    opacity: 0.3;
    animation: drift-c 30s ease-in-out infinite alternate;
  }
  .idle .a {
    background: var(--violet);
  }
  .idle .b {
    background: var(--blue);
    opacity: 0.4;
  }
  .idle .c {
    background: var(--pink);
  }
  .live .a {
    background: var(--mint);
    opacity: 0.42;
  }
  .live .b {
    background: var(--cyan);
    opacity: 0.38;
  }
  .live .c {
    background: var(--mint);
  }
  .grain {
    position: absolute;
    inset: 0;
    opacity: 0.05;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
  }
  .vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(140% 100% at 50% 0%, transparent 55%, rgba(0, 0, 0, 0.45) 100%);
  }
  @keyframes drift-a {
    to {
      transform: translate(120px, 60px) scale(1.15);
    }
  }
  @keyframes drift-b {
    to {
      transform: translate(-140px, 80px) scale(0.9);
    }
  }
  @keyframes drift-c {
    to {
      transform: translate(-100px, -40px) scale(1.2);
    }
  }
</style>
