<script lang="ts" generics="T extends string">
  // Segmented control with a sliding pill (transitions.dev "Tabs sliding").
  import { onMount } from "svelte";

  let {
    options,
    value,
    onchange,
    size = "m",
  }: {
    options: { value: T; label: string; dot?: boolean }[];
    value: T;
    onchange: (v: T) => void;
    size?: "s" | "m";
  } = $props();

  let bar: HTMLDivElement;
  let pill: HTMLSpanElement;
  let ready = $state(false);

  function place(animate: boolean) {
    const tab = bar?.querySelector<HTMLButtonElement>(`[data-value="${CSS.escape(value)}"]`);
    if (!tab || !pill) return;
    if (!animate) pill.style.transition = "none";
    pill.style.transform = `translateX(${tab.offsetLeft}px)`;
    pill.style.width = `${tab.offsetWidth}px`;
    if (!animate) {
      void pill.offsetWidth;
      pill.style.transition = "";
    }
  }

  $effect(() => {
    value; // re-run when the selection changes
    place(ready);
  });

  onMount(() => {
    requestAnimationFrame(() => {
      place(false);
      ready = true;
    });
    const ro = new ResizeObserver(() => place(false));
    ro.observe(bar);
    return () => ro.disconnect();
  });
</script>

<div class="t-tabs {size}" role="tablist" bind:this={bar}>
  <span class="t-tabs-pill" aria-hidden="true" bind:this={pill}></span>
  {#each options as o (o.value)}
    <button
      class="t-tab"
      role="tab"
      data-value={o.value}
      aria-selected={o.value === value}
      onclick={() => o.value !== value && onchange(o.value)}
    >
      {#if o.dot}<span class="dot"></span>{/if}{o.label}
    </button>
  {/each}
</div>

<style>
  .t-tabs {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: rgba(0, 0, 0, 0.28);
    border: 1px solid var(--stroke);
  }
  .t-tab {
    position: relative;
    z-index: 1;
    height: 30px;
    padding: 0 14px;
    border-radius: 999px;
    color: var(--text-3);
    font-weight: 550;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    white-space: nowrap;
    transition: color var(--duration-fast) var(--ease-smooth-out);
  }
  .s .t-tab {
    height: 26px;
    padding: 0 11px;
    font-size: 13px;
  }
  .t-tab:hover,
  .t-tab[aria-selected="true"] {
    color: var(--text);
  }
  .t-tabs-pill {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    width: 0;
    border-radius: 999px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.13), rgba(255, 255, 255, 0.07));
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.12),
      0 4px 14px -4px rgba(0, 0, 0, 0.5);
    transition:
      transform var(--duration-fast) var(--ease-smooth-out),
      width var(--duration-fast) var(--ease-smooth-out);
    will-change: transform, width;
    pointer-events: none;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mint);
    box-shadow: 0 0 8px var(--mint);
  }
  @media (prefers-reduced-motion: reduce) {
    .t-tabs-pill,
    .t-tab {
      transition: none !important;
    }
  }
</style>
