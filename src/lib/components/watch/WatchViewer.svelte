<script lang="ts">
  // Live view of exactly what Screen Watch sees in a region, what it read,
  // and whether the condition matches right now.
  import { onDestroy } from "svelte";
  import { api, errorText, type Condition, type Region, type ViewerCheck } from "$lib/api";

  let {
    region,
    condition,
    active = true,
    onpickcolor,
  }: {
    region: Region | null;
    condition: Condition;
    active?: boolean;
    /** Colour conditions: clicking the preview picks that pixel. */
    onpickcolor?: (dx: number, dy: number, rgb: [number, number, number]) => void;
  } = $props();

  const TICK_MS = 300;
  const MAX_W = 560;

  let canvas: HTMLCanvasElement | undefined = $state();
  let check = $state<ViewerCheck | null>(null);
  let error = $state("");
  let stats = $state({ checks: 0, ocrRuns: 0 });
  let busy = false;

  let valid = $derived(!!region && region.w > 0 && region.h > 0);
  let usesOcr = $derived(condition.kind === "textAppears" || condition.kind === "textGone" || condition.kind === "number");

  // Reset counters whenever what we're watching changes.
  $effect(() => {
    JSON.stringify([region, condition]);
    stats = { checks: 0, ocrRuns: 0 };
    check = null;
  });

  async function tick() {
    if (busy || !region || document.hidden) return;
    busy = true;
    try {
      const c = await api.viewerCheck($state.snapshot(region), $state.snapshot(condition));
      check = c;
      error = "";
      stats = { checks: stats.checks + 1, ocrRuns: stats.ocrRuns + (c.ocrRan ? 1 : 0) };
      draw(await api.viewerFrame(MAX_W));
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  function draw(raw: ArrayBuffer | number[]) {
    const buf = raw instanceof ArrayBuffer ? raw : new Uint8Array(raw).buffer;
    if (!canvas || buf.byteLength < 8) return;
    const dv = new DataView(buf);
    const w = dv.getUint32(0, true);
    const h = dv.getUint32(4, true);
    if (buf.byteLength < 8 + w * h * 4) return;
    canvas.width = w;
    canvas.height = h;
    canvas.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(buf, 8, w * h * 4), w, h), 0, 0);
  }

  let timer: ReturnType<typeof setInterval> | undefined;
  $effect(() => {
    clearInterval(timer);
    if (active && valid) {
      tick();
      timer = setInterval(tick, TICK_MS);
    }
    return () => clearInterval(timer);
  });
  onDestroy(() => {
    clearInterval(timer);
    api.viewerStop().catch(() => {});
  });

  function pick(e: MouseEvent) {
    if (!onpickcolor || !canvas || !region) return;
    const rect = canvas.getBoundingClientRect();
    const fx = (e.clientX - rect.left) / rect.width;
    const fy = (e.clientY - rect.top) / rect.height;
    const px = canvas.getContext("2d")!.getImageData(Math.floor(fx * canvas.width), Math.floor(fy * canvas.height), 1, 1).data;
    onpickcolor(Math.floor(fx * region.w), Math.floor(fy * region.h), [px[0], px[1], px[2]]);
  }

  let marker = $derived(
    condition.kind === "colorAt" && region
      ? { left: `${((condition.dx + 0.5) / region.w) * 100}%`, top: `${((condition.dy + 0.5) / region.h) * 100}%` }
      : null,
  );

  const fmtNum = (n: number) => (Math.abs(n) >= 1e9 ? n.toExponential(3).replace(/\.?0+e/, "e") : n.toLocaleString());
</script>

<div class="viewer" class:matched={check?.matched}>
  <div class="screen" class:empty={!valid}>
    {#if valid}
      <span class="corner tl"></span><span class="corner tr"></span><span class="corner bl"></span><span class="corner br"></span>
      <div class="img-wrap">
        <canvas bind:this={canvas} class:pickable={!!onpickcolor} onclick={pick}></canvas>
        {#if marker}<span class="marker" style:left={marker.left} style:top={marker.top}></span>{/if}
      </div>
    {:else}
      <p>Select an area of the screen to see it here, live.</p>
    {/if}
  </div>

  {#if valid}
    <div class="readout">
      {#if error}
        <div class="line err">{error}</div>
      {:else}
        {#if usesOcr}
          <div class="line">
            <span>Reads</span>
            <b class="text">{check?.text?.trim() ? check.text : check ? "(no text found)" : "…"}</b>
          </div>
        {/if}
        {#if condition.kind === "number"}
          <div class="line">
            <span>Numbers</span>
            <b class="nums">
              {#each check?.numbers ?? [] as n}
                <span class="num" class:hit={check?.matched && n === check.number}>{fmtNum(n)}</span>
              {:else}
                none found
              {/each}
            </b>
          </div>
        {/if}
        <div class="line state" class:yes={check?.matched}>
          <span class="dot"></span>
          <b>{check ? (check.matched ? "Matches right now" : "Doesn't match yet") : "Checking…"}</b>
          {#if onpickcolor}<small>Click the preview to pick a colour</small>{/if}
        </div>
        <div class="stats">
          capture {check ? check.captureMs.toFixed(1) : "–"} ms
          {#if usesOcr}
            · OCR re-read {stats.ocrRuns} of {stats.checks} checks{check?.ocrRan ? ` (${check.ocrMs.toFixed(0)} ms)` : ""}
          {/if}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .screen {
    position: relative;
    padding: 10px;
    display: grid;
    place-items: center;
    min-height: 110px;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.3);
  }
  .screen.empty {
    border: 1px dashed var(--stroke-strong);
  }
  .screen.empty p {
    color: var(--text-3);
    font-size: 13px;
  }
  .corner {
    position: absolute;
    width: 16px;
    height: 16px;
    border: 2px solid var(--cyan);
    transition: border-color 300ms var(--ease-smooth-out);
  }
  .matched .corner {
    border-color: var(--mint);
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
  .img-wrap {
    position: relative;
    max-width: 100%;
  }
  canvas {
    display: block;
    max-width: 100%;
    max-height: 220px;
    border-radius: 6px;
    image-rendering: pixelated;
  }
  canvas.pickable {
    cursor: crosshair;
  }
  .marker {
    position: absolute;
    width: 14px;
    height: 14px;
    translate: -50% -50%;
    border-radius: 50%;
    border: 2px solid white;
    box-shadow:
      0 0 0 2px rgba(0, 0, 0, 0.6),
      0 0 10px rgba(255, 255, 255, 0.6);
    pointer-events: none;
  }
  .readout {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
  }
  .line {
    display: flex;
    gap: 12px;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--stroke);
  }
  .line span {
    color: var(--text-3);
    flex: none;
  }
  .line .text {
    text-align: right;
    white-space: pre-wrap;
    word-break: break-word;
    font-weight: 600;
  }
  .nums {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
  .num {
    padding: 0 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.06);
    font-variant-numeric: tabular-nums;
  }
  .num.hit {
    color: #052e22;
    background: var(--mint);
  }
  .line.err {
    color: #ffc2ca;
    border-color: rgba(255, 92, 112, 0.4);
  }
  .state {
    justify-content: flex-start;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    transition: all 250ms var(--ease-smooth-out);
  }
  .state .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .state.yes {
    color: var(--mint);
    border-color: rgba(61, 242, 176, 0.35);
    background: rgba(61, 242, 176, 0.07);
  }
  .state.yes .dot {
    background: var(--mint);
    box-shadow: 0 0 10px var(--mint);
  }
  .state small {
    margin-left: auto;
    color: var(--text-3);
    font-size: 11.5px;
  }
  .stats {
    font-size: 11.5px;
    color: var(--text-3);
    padding: 0 4px;
    font-variant-numeric: tabular-nums;
  }
</style>
