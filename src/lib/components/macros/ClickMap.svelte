<script lang="ts">
  // A miniature of your monitors with a dot for every recorded click (the
  // newest one pulses) and faint lines for recorded mouse movement.
  import { onMount } from "svelte";
  import { api, type Region, type Step } from "$lib/api";

  let { steps }: { steps: Step[] } = $props();

  let monitors = $state<Region[]>([]);
  onMount(async () => {
    monitors = await api.screenLayout().catch(() => []);
  });

  let bounds = $derived.by(() => {
    if (monitors.length === 0) return { x: 0, y: 0, w: 1920, h: 1080 };
    const x0 = Math.min(...monitors.map((m) => m.x));
    const y0 = Math.min(...monitors.map((m) => m.y));
    const x1 = Math.max(...monitors.map((m) => m.x + m.w));
    const y1 = Math.max(...monitors.map((m) => m.y + m.h));
    return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
  });

  let clicks = $derived(
    steps.flatMap((s) =>
      s.type === "click" && s.at && s.action !== "up" ? [{ x: s.at[0], y: s.at[1], right: s.button === "RightMouse" }] : [],
    ),
  );
  let paths = $derived(
    steps.flatMap((s) => (s.type === "path" ? [s.points.map((p) => `${p.x},${p.y}`).join(" ")] : [])),
  );
  let r = $derived(bounds.w / 140);
</script>

<svg class="map" viewBox="{bounds.x} {bounds.y} {bounds.w} {bounds.h}" preserveAspectRatio="xMidYMid meet">
  {#each monitors as m}
    <rect x={m.x + r} y={m.y + r} width={m.w - 2 * r} height={m.h - 2 * r} rx={r * 3} class="mon" style:stroke-width={r / 3} />
  {/each}
  {#each paths as p}
    <polyline points={p} class="path" style:stroke-width={r / 2.5} />
  {/each}
  {#each clicks as c, i}
    {@const last = i === clicks.length - 1}
    {#if last}<circle cx={c.x} cy={c.y} r={r * 2.4} class="pulse" style:stroke-width={r / 3} />{/if}
    <circle cx={c.x} cy={c.y} r={last ? r * 1.2 : r * 0.85} class="dot" class:right={c.right} class:last />
  {/each}
</svg>

<style>
  .map {
    display: block;
    width: 100%;
    height: 100%;
  }
  .mon {
    fill: rgba(255, 255, 255, 0.035);
    stroke: rgba(255, 255, 255, 0.14);
  }
  .path {
    fill: none;
    stroke: rgba(91, 140, 255, 0.55);
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dot {
    fill: rgba(255, 110, 199, 0.75);
  }
  .dot.right {
    fill: rgba(56, 214, 255, 0.8);
  }
  .dot.last {
    fill: white;
  }
  .pulse {
    fill: none;
    stroke: var(--pink);
    transform-box: fill-box;
    transform-origin: center;
    animation: ping 1.4s var(--ease-smooth-out) infinite;
  }
  @keyframes ping {
    from {
      transform: scale(0.4);
      opacity: 1;
    }
    to {
      transform: scale(1.6);
      opacity: 0;
    }
  }
</style>
