<script lang="ts">
  // What's been recorded so far, updating live: a click map plus the most
  // recent steps, newest at the bottom.
  import type { Step } from "$lib/api";
  import { describe, formatMs } from "$lib/steps";
  import StepIcon from "../ui/StepIcon.svelte";
  import ClickMap from "./ClickMap.svelte";

  let { steps, compact = false }: { steps: Step[]; compact?: boolean } = $props();

  type Row = { step: Step; index: number; after: number };
  // Waits become "+250 ms" on the action that follows them.
  let rows = $derived.by(() => {
    const out: Row[] = [];
    let gap = 0;
    steps.forEach((s, index) => {
      if (s.type === "wait") gap += s.ms;
      else {
        out.push({ step: s, index, after: gap });
        gap = 0;
      }
    });
    return out;
  });
  let visible = $derived(rows.slice(-(compact ? 6 : 8)));
  let clicks = $derived(steps.filter((s) => s.type === "click" && s.action !== "up").length);
  let keys = $derived(steps.filter((s) => s.type === "key" && s.action !== "up").length);
</script>

<div class="live" class:compact>
  <div class="map-wrap"><ClickMap {steps} /></div>

  <div class="counts">
    <span><b>{rows.length}</b> actions</span>
    <span><b>{clicks}</b> clicks</span>
    <span><b>{keys}</b> keys</span>
  </div>

  <ol class="rows">
    {#if rows.length > visible.length}
      <li class="more">+ {rows.length - visible.length} earlier</li>
    {/if}
    {#each visible as r (r.index)}
      <li class="row">
        <StepIcon type={r.step.type} size={compact ? 22 : 26} />
        <span class="what">{describe(r.step)}</span>
        {#if r.after > 0}<span class="after">+{formatMs(r.after)}</span>{/if}
      </li>
    {:else}
      <li class="empty">Waiting for your first click or key…</li>
    {/each}
  </ol>
</div>

<style>
  .live {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .map-wrap {
    height: 150px;
    padding: 8px;
    border-radius: 14px;
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--stroke);
  }
  .compact .map-wrap {
    height: 110px;
  }
  .counts {
    display: flex;
    gap: 16px;
    font-size: 12px;
    color: var(--text-3);
    padding: 0 4px;
  }
  .counts b {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px 6px 6px;
    border-radius: 11px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--stroke);
    font-size: 13px;
    font-weight: 550;
    animation: enter 350ms var(--ease-smooth-out) both;
  }
  .row:last-child {
    border-color: rgba(255, 92, 112, 0.4);
    background: rgba(255, 92, 112, 0.06);
  }
  .what {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .after {
    font-size: 11.5px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .more,
  .empty {
    font-size: 12px;
    color: var(--text-3);
    padding: 2px 6px;
  }
  .compact .row {
    font-size: 12.5px;
  }
  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(8px);
      filter: blur(2px);
    }
  }
</style>
