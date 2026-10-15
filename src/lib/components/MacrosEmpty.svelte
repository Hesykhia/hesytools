<script lang="ts">
  // Placeholder for the macro library until the recorder lands. The ghost
  // steps preview what a recorded macro will look like.
  const STEPS = [
    { icon: "key", label: "Press", value: "E" },
    { icon: "wait", label: "Wait", value: "200 ms" },
    { icon: "eye", label: "Wait until screen shows", value: "“Ready”" },
    { icon: "click", label: "Left click at", value: "940, 494" },
  ];
  const ICONS: Record<string, string> = {
    key: "M5 7h14a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2zm3 7h8",
    wait: "M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
    eye: "M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12zm10 2.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5z",
    click: "M9 3v3M4.5 5.5l2 2M3 10h3m3-1 10 4-4 1.5L13.5 19z",
  };
</script>

<section class="macros">
  <div class="head">
    <div>
      <h2>Macros</h2>
      <p>Record what you do once, replay it forever.</p>
    </div>
    <button class="btn" disabled title="Coming in the next update">
      <span class="rec"></span>Record a macro
    </button>
  </div>

  <div class="empty glass">
    <div class="ghosts" aria-hidden="true">
      {#each STEPS as s, i}
        <div class="step" style:--i={i}>
          <span class="n">{i + 1}</span>
          <span class="ic {s.icon}"
            ><svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d={ICONS[s.icon]} /></svg
          ></span>
          <span class="lbl">{s.label}</span>
          <span class="val">{s.value}</span>
        </div>
      {/each}
    </div>
    <div class="copy">
      <span class="badge">Next up</span>
      <h3>Your macros will live here</h3>
      <p>
        Hit record and play normally. hesytools captures your keys, clicks and timing as editable steps. Then add loops,
        conditions, and "wait until this appears on screen".
      </p>
    </div>
  </div>
</section>

<style>
  .macros {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 0 6px;
  }
  h2 {
    font-size: 19px;
    letter-spacing: -0.02em;
  }
  .head p {
    color: var(--text-3);
    font-size: 13px;
    margin-top: 2px;
  }
  .rec {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 8px var(--red);
  }
  .empty {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 30px;
    align-items: center;
    padding: 26px 30px;
  }
  .ghosts {
    display: flex;
    flex-direction: column;
    gap: 8px;
    mask-image: linear-gradient(180deg, #000 55%, transparent);
  }
  .step {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--stroke);
    font-size: 13px;
    animation: rise 500ms var(--ease-smooth-out) both;
    animation-delay: calc(var(--i) * 70ms + 100ms);
  }
  .n {
    width: 18px;
    color: var(--text-3);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .ic {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    flex: none;
    display: grid;
    place-items: center;
  }
  .ic.key {
    color: var(--violet-2);
    background: rgba(139, 108, 255, 0.22);
  }
  .ic.wait {
    color: var(--text-2);
    background: rgba(255, 255, 255, 0.1);
  }
  .ic.eye {
    color: var(--cyan);
    background: rgba(56, 214, 255, 0.18);
  }
  .ic.click {
    color: var(--pink);
    background: rgba(255, 110, 199, 0.18);
  }
  .lbl {
    color: var(--text-2);
  }
  .val {
    margin-left: auto;
    font-weight: 600;
  }
  .badge {
    display: inline-block;
    font-size: 11px;
    font-weight: 650;
    color: var(--violet-2);
    background: rgba(139, 108, 255, 0.14);
    border: 1px solid rgba(139, 108, 255, 0.35);
    padding: 2px 9px;
    border-radius: 99px;
    margin-bottom: 10px;
  }
  h3 {
    font-size: 18px;
    letter-spacing: -0.02em;
    margin-bottom: 6px;
  }
  .copy p {
    color: var(--text-2);
    line-height: 1.55;
    font-size: 13.5px;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(12px);
      filter: blur(3px);
    }
  }
</style>
