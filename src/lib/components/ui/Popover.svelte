<script lang="ts">
  // Anchored popover that grows from its trigger (transitions.dev
  // "Menu dropdown": 250ms open from 0.97, 150ms close to 0.99).
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    trigger,
    children,
    width = 300,
    align = "start",
  }: {
    open?: boolean;
    trigger: Snippet<[{ toggle: () => void; open: boolean }]>;
    children: Snippet<[{ close: () => void }]>;
    width?: number;
    align?: "start" | "center";
  } = $props();

  let root: HTMLSpanElement;
  let closing = $state(false);
  let shown = $state(false);

  function toggle() {
    if (open) close();
    else {
      open = true;
    }
  }

  function close() {
    if (!open) return;
    closing = true;
    setTimeout(() => {
      open = false;
      closing = false;
    }, 150);
  }

  $effect(() => {
    if (open) {
      shown = false;
      requestAnimationFrame(() => (shown = true));
    }
  });

  function onWindowPointer(e: PointerEvent) {
    if (open && !root.contains(e.target as Node)) close();
  }
  function onKey(e: KeyboardEvent) {
    if (open && e.key === "Escape") close();
  }
</script>

<svelte:window onpointerdown={onWindowPointer} onkeydown={onKey} />

<span class="anchor" bind:this={root}>
  {@render trigger({ toggle, open })}
  {#if open}
    <div
      class="pop glass {align}"
      class:shown={shown && !closing}
      class:closing
      style:width="{width}px"
      role="dialog"
    >
      {@render children({ close })}
    </div>
  {/if}
</span>

<style>
  .anchor {
    position: relative;
    display: inline-flex;
  }
  .pop {
    position: absolute;
    top: calc(100% + 10px);
    left: -6px;
    z-index: 40;
    padding: 14px;
    border-radius: 18px;
    background: var(--popover);
    border-color: var(--stroke-strong);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.07),
      0 24px 60px -12px rgba(0, 0, 0, 0.75);
    transform-origin: top left;
    opacity: 0;
    transform: scale(0.97) translateY(-4px);
    transition:
      opacity 250ms var(--ease-smooth-out),
      transform 250ms var(--ease-smooth-out);
  }
  .pop.center {
    left: 50%;
    translate: -50% 0;
    transform-origin: top center;
  }
  .pop.shown {
    opacity: 1;
    transform: scale(1) translateY(0);
  }
  .pop.closing {
    opacity: 0;
    transform: scale(0.99);
    transition-duration: 150ms;
  }
  @media (prefers-reduced-motion: reduce) {
    .pop {
      transition: none !important;
    }
  }
</style>
