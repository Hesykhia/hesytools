<script lang="ts">
  import Digits from "../ui/Digits.svelte";

  let {
    phase,
    cps,
    countdown,
    onclick,
  }: {
    phase: "idle" | "countdown" | "running";
    cps: number;
    countdown: number;
    onclick: () => void;
  } = $props();

  let cpsText = $derived(cps >= 10 ? Math.round(cps).toString() : cps.toFixed(1));
</script>

<button class="orb {phase}" {onclick} aria-label={phase === "running" ? "Stop clicking" : "Start clicking"}>
  <span class="halo"></span>
  <span class="sonar s1"></span>
  <span class="sonar s2"></span>
  <span class="ring"></span>
  <span class="core">
    {#if phase === "running"}
      <span class="big"><Digits value={cpsText} /></span>
      <span class="unit">clicks / sec</span>
      <span class="hint">click to stop</span>
    {:else if phase === "countdown"}
      {#key countdown}<span class="big count">{countdown}</span>{/key}
      <span class="unit">get ready</span>
      <span class="hint">click to cancel</span>
    {:else}
      <svg class="play" width="34" height="34" viewBox="0 0 24 24"><path d="M8 5.5v13a1 1 0 0 0 1.5.87l11-6.5a1 1 0 0 0 0-1.74l-11-6.5A1 1 0 0 0 8 5.5z" fill="currentColor" /></svg>
      <span class="label">Start</span>
    {/if}
  </span>
</button>

<style>
  .orb {
    --c1: var(--violet);
    --c2: var(--pink);
    --c3: var(--blue);
    --glow: rgba(139, 108, 255, 0.55);
    position: relative;
    width: 216px;
    height: 216px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    transition: transform 350ms var(--ease-bounce);
  }
  .orb:hover {
    transform: scale(1.025);
  }
  .orb:active {
    transform: scale(0.97);
    transition-duration: 120ms;
  }
  .orb.running {
    --c1: var(--mint);
    --c2: var(--cyan);
    --c3: #a6ffdf;
    --glow: rgba(61, 242, 176, 0.5);
  }
  .orb.countdown {
    --c1: var(--amber);
    --c2: var(--pink);
    --c3: var(--violet);
    --glow: rgba(255, 191, 92, 0.45);
  }

  .halo {
    position: absolute;
    inset: 18px;
    border-radius: 50%;
    background: radial-gradient(circle, var(--glow), transparent 70%);
    filter: blur(30px);
    opacity: 0.8;
    transition:
      opacity 400ms var(--ease-smooth-out),
      background 600ms var(--ease-in-out);
  }
  .orb:hover .halo {
    opacity: 1;
  }
  .running .halo {
    animation: breathe 2.4s ease-in-out infinite;
  }

  .ring {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    padding: 2px;
    background: conic-gradient(from 0deg, var(--c1), var(--c2), var(--c3), var(--c1));
    -webkit-mask:
      linear-gradient(#000 0 0) content-box,
      linear-gradient(#000 0 0);
    mask:
      linear-gradient(#000 0 0) content-box,
      linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
    opacity: 0.9;
    animation: spin 14s linear infinite;
  }
  .running .ring {
    animation-duration: 2.2s;
    padding: 3px;
  }

  .core {
    position: absolute;
    inset: 10px;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    background:
      radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.12), transparent 60%),
      radial-gradient(circle at 50% 120%, var(--glow), transparent 60%),
      #0e0d16;
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      inset 0 0 40px rgba(0, 0, 0, 0.6),
      0 20px 40px -10px rgba(0, 0, 0, 0.7);
    transition: background 600ms var(--ease-in-out);
  }

  .play {
    color: white;
    margin-left: 4px;
    filter: drop-shadow(0 0 12px var(--glow));
  }
  .label {
    font-weight: 650;
    font-size: 15px;
    letter-spacing: 0.01em;
    margin-top: 6px;
  }
  .big {
    font-size: 54px;
    font-weight: 700;
    letter-spacing: -0.04em;
    line-height: 1;
    background: linear-gradient(180deg, #fff, rgba(255, 255, 255, 0.7));
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .count {
    display: inline-block;
    animation: pop 500ms cubic-bezier(0.34, 1.45, 0.64, 1) both;
  }
  .unit {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    letter-spacing: 0.02em;
  }
  .hint {
    position: absolute;
    bottom: 34px;
    font-size: 11px;
    color: var(--text-3);
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-smooth-out);
  }
  .orb:hover .hint {
    opacity: 1;
  }

  .sonar {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1px solid var(--c1);
    opacity: 0;
  }
  .running .sonar {
    animation: sonar 2.4s var(--ease-smooth-out) infinite;
  }
  .running .s2 {
    animation-delay: 1.2s;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes breathe {
    50% {
      opacity: 0.55;
      transform: scale(0.94);
    }
  }
  @keyframes sonar {
    0% {
      transform: scale(1);
      opacity: 0.5;
    }
    100% {
      transform: scale(1.45);
      opacity: 0;
    }
  }
  @keyframes pop {
    0% {
      transform: translateY(10px) scale(0.9);
      opacity: 0;
      filter: blur(3px);
    }
    100% {
      transform: none;
      opacity: 1;
      filter: blur(0);
    }
  }
</style>
