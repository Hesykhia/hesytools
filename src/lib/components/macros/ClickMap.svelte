<script lang="ts">
  // A miniature of your monitors with a dot for every click in `steps` and
  // faint lines for recorded mouse movement. While recording the newest
  // click pulses; in the editor the playing step's click does.
  import { onMount } from "svelte";
  import { api, type Region, type Step } from "$lib/api";

  let {
    steps,
    active = "last",
    numbered = false,
  }: {
    steps: Step[];
    /** Top-level step index to highlight, or "last" for the newest click. */
    active?: number | "last";
    /** Label dots with their action number (matches the step list). */
    numbered?: boolean;
  } = $props();

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

  type Dot = { x: number; y: number; right: boolean; owner: number; n: number };
  // Clicks with a position, including those inside loops (owned by the loop's
  // top-level step). `n` is the action number shown in the step list.
  let clicks = $derived.by(() => {
    const out: Dot[] = [];
    let n = 0;
    steps.forEach((s, owner) => {
      if (s.type !== "wait") n++;
      const visit = (st: Step) => {
        if (st.type === "click" && st.at && st.action !== "up")
          out.push({ x: st.at[0], y: st.at[1], right: st.button === "RightMouse", owner, n });
        if (st.type === "repeat") st.steps.forEach(visit);
      };
      visit(s);
    });
    return out;
  });
  let paths = $derived(
    steps.flatMap((s) => (s.type === "path" ? [s.points.map((p) => `${p.x},${p.y}`).join(" ")] : [])),
  );
  let r = $derived(bounds.w / 140);
  let isHot = (c: Dot, i: number) => (active === "last" ? i === clicks.length - 1 : c.owner === active);
</script>

<svg class="map" viewBox="{bounds.x} {bounds.y} {bounds.w} {bounds.h}" preserveAspectRatio="xMidYMid meet">
  {#each monitors as m}
    <rect x={m.x + r} y={m.y + r} width={m.w - 2 * r} height={m.h - 2 * r} rx={r * 3} class="mon" style:stroke-width={r / 3} />
  {/each}
  {#each paths as p}
    <polyline points={p} class="path" style:stroke-width={r / 2.5} />
  {/each}
  {#each clicks as c, i}
    {@const hot = isHot(c, i)}
    {#if hot}<circle cx={c.x} cy={c.y} r={r * 2.4} class="pulse" style:stroke-width={r / 3} />{/if}
    <circle cx={c.x} cy={c.y} r={hot ? r * 1.2 : r * 0.85} class="dot" class:right={c.right} class:hot />
    {#if numbered}
      <text x={c.x + r * 1.6} y={c.y - r * 1.2} class="label" style:font-size="{r * 2.6}px">{c.n}</text>
    {/if}
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
    transition: r 200ms var(--ease-smooth-out);
  }
  .dot.right {
    fill: rgba(56, 214, 255, 0.8);
  }
  .dot.hot {
    fill: white;
  }
  .label {
    fill: var(--text-2);
    font-weight: 700;
    font-family: inherit;
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
