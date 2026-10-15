<script lang="ts">
  // Expand/collapse via grid-rows (transitions.dev "Accordion expand").
  import type { Snippet } from "svelte";
  let { title, open = $bindable(false), children }: { title: string; open?: boolean; children: Snippet } = $props();
</script>

<div class="acc" class:open>
  <button class="head" aria-expanded={open} onclick={() => (open = !open)}>
    <span>{title}</span>
    <svg class="chev" width="12" height="12" viewBox="0 0 12 12"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" /></svg>
  </button>
  <div class="body">
    <div class="inner">{@render children()}</div>
  </div>
</div>

<style>
  .head {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
    font-weight: 550;
    font-size: 13px;
    padding: 4px 0;
    transition: color var(--duration-quick) var(--ease-smooth-out);
  }
  .head:hover,
  .open .head {
    color: var(--text-2);
  }
  .chev {
    transition: transform 250ms var(--ease-smooth-out);
  }
  .open .chev {
    transform: rotate(180deg);
  }
  .body {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows 250ms var(--ease-smooth-out);
  }
  .open .body {
    grid-template-rows: 1fr;
  }
  .inner {
    overflow: hidden;
    opacity: 0;
    transition: opacity 250ms var(--ease-smooth-out);
  }
  .open .inner {
    opacity: 1;
  }
</style>
