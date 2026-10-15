<script lang="ts">
  // Preview of Screen Watch: a mock of the live viewer showing what the
  // OCR region "sees", plus the kinds of triggers it will support.
  const FEATURES = [
    { title: "Read text & numbers", body: "Wait until “Ready” shows up, or a value goes past 1e+25.", tint: "violet" },
    { title: "Match a colour", body: "React the moment a pixel turns red, like a health bar.", tint: "pink" },
    { title: "Find an image", body: "Spot a button or icon anywhere inside the region.", tint: "cyan" },
  ];
</script>

<div class="wrap">
  <section class="hero glass">
    <div class="copy">
      <span class="badge">Coming soon</span>
      <h2>Screen Watch</h2>
      <p>
        Draw a box anywhere on screen and hesytools watches it for you, with a live view of exactly what it's reading. It only
        re-reads when the pixels actually change, so it barely touches your CPU.
      </p>
    </div>

    <div class="viewer" aria-hidden="true">
      <div class="frame">
        <span class="corner tl"></span><span class="corner tr"></span><span class="corner bl"></span><span class="corner br"></span>
        <div class="screen">
          <span class="game">WAVE 12</span>
          <span class="game big">1.25e+25</span>
          <span class="scan"></span>
        </div>
      </div>
      <div class="readout">
        <div class="line"><span>Reads</span><b>WAVE 12 · 1.25e+25</b></div>
        <div class="line"><span>Condition</span><b>number ≥ 1e+25</b></div>
        <div class="line match"><span class="pulse"></span><b>Match, running “Rebirth”</b></div>
      </div>
    </div>
  </section>

  <div class="features">
    {#each FEATURES as f, i}
      <div class="feature glass {f.tint}" style:--i={i}>
        <span class="glow"></span>
        <h3>{f.title}</h3>
        <p>{f.body}</p>
      </div>
    {/each}
  </div>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .hero {
    display: grid;
    grid-template-columns: 1fr 1.1fr;
    gap: 30px;
    align-items: center;
    padding: 32px;
  }
  .badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 650;
    color: var(--cyan);
    background: rgba(56, 214, 255, 0.12);
    border: 1px solid rgba(56, 214, 255, 0.35);
    padding: 2px 9px;
    border-radius: 99px;
    margin-bottom: 12px;
  }
  h2 {
    font-size: 28px;
    letter-spacing: -0.03em;
    margin-bottom: 10px;
  }
  .copy p {
    color: var(--text-2);
    line-height: 1.6;
  }
  .frame {
    position: relative;
    padding: 10px;
  }
  .corner {
    position: absolute;
    width: 18px;
    height: 18px;
    border: 2px solid var(--cyan);
    filter: drop-shadow(0 0 6px var(--cyan));
  }
  .tl {
    top: 0;
    left: 0;
    border-right: 0;
    border-bottom: 0;
    border-top-left-radius: 8px;
  }
  .tr {
    top: 0;
    right: 0;
    border-left: 0;
    border-bottom: 0;
    border-top-right-radius: 8px;
  }
  .bl {
    bottom: 0;
    left: 0;
    border-right: 0;
    border-top: 0;
    border-bottom-left-radius: 8px;
  }
  .br {
    bottom: 0;
    right: 0;
    border-left: 0;
    border-top: 0;
    border-bottom-right-radius: 8px;
  }
  .screen {
    position: relative;
    overflow: hidden;
    height: 120px;
    border-radius: 10px;
    background:
      radial-gradient(120% 120% at 20% 0%, rgba(255, 110, 199, 0.25), transparent 60%),
      radial-gradient(100% 100% at 100% 100%, rgba(91, 140, 255, 0.3), transparent 60%),
      #0b0b14;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
  }
  .game {
    font-weight: 800;
    letter-spacing: 0.08em;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
  }
  .game.big {
    font-size: 30px;
    letter-spacing: -0.02em;
    color: white;
    text-shadow: 0 0 20px rgba(255, 255, 255, 0.35);
  }
  .scan {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 40px;
    background: linear-gradient(180deg, transparent, rgba(56, 214, 255, 0.22), transparent);
    transform: translateY(-40px);
    /* A short demo, then it rests (no endless redraws while idle). */
    animation: scan 2.6s ease-in-out 4;
  }
  .readout {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12.5px;
  }
  .line {
    display: flex;
    justify-content: space-between;
    padding: 7px 12px;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--stroke);
  }
  .line span {
    color: var(--text-3);
  }
  .line.match {
    justify-content: flex-start;
    gap: 8px;
    color: var(--mint);
    border-color: rgba(61, 242, 176, 0.3);
    background: rgba(61, 242, 176, 0.06);
  }
  .pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    align-self: center;
    background: var(--mint) !important;
    box-shadow: 0 0 10px var(--mint);
    animation: blink 1.4s ease-in-out 4;
  }
  .features {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
  }
  .feature {
    position: relative;
    overflow: hidden;
    padding: 20px;
    animation: rise 500ms var(--ease-smooth-out) both;
    animation-delay: calc(var(--i) * 60ms + 80ms);
  }
  .glow {
    position: absolute;
    width: 260px;
    height: 260px;
    top: -150px;
    right: -110px;
    border-radius: 50%;
    opacity: 0.35;
  }
  .violet .glow {
    background: radial-gradient(circle, var(--violet), transparent 65%);
  }
  .pink .glow {
    background: radial-gradient(circle, var(--pink), transparent 65%);
  }
  .cyan .glow {
    background: radial-gradient(circle, var(--cyan), transparent 65%);
  }
  .feature h3 {
    position: relative;
    font-size: 15px;
    margin-bottom: 6px;
  }
  .feature p {
    position: relative;
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.5;
  }
  @keyframes scan {
    0% {
      transform: translateY(-40px);
    }
    100% {
      transform: translateY(120px);
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.35;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(12px);
      filter: blur(3px);
    }
  }
</style>
