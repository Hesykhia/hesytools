<script lang="ts">
  // Number that pops changed digits in with a blurred rise
  // (transitions.dev "Number pop-in"). Unchanged leading digits stay still.
  let { value }: { value: string } = $props();

  let chars = $state<{ ch: string; same: boolean; stagger: number }[]>([]);
  let gen = $state(0);
  let prev = "";

  $effect(() => {
    const next = value;
    if (next === prev) return;
    let same = 0;
    while (same < prev.length && same < next.length && prev[same] === next[same]) same++;
    // Keep it alive but not chaotic: only the last two digits stagger.
    chars = next.split("").map((ch, i) => ({
      ch,
      same: i < same && prev.length === next.length,
      stagger: i === next.length - 1 ? 2 : i === next.length - 2 ? 1 : 0,
    }));
    prev = next;
    gen++;
  });
</script>

{#key gen}
  <span class="t-digit-group is-animating">
    {#each chars as c}
      <span class="t-digit" data-same={c.same || undefined} data-stagger={c.stagger || undefined}>{c.ch}</span>
    {/each}
  </span>
{/key}

<style>
  .t-digit-group {
    display: inline-flex;
    align-items: baseline;
    font-variant-numeric: tabular-nums;
  }
  .t-digit {
    display: inline-block;
    will-change: transform, opacity, filter;
  }
  .is-animating .t-digit {
    animation: t-digit-pop-in 500ms cubic-bezier(0.34, 1.45, 0.64, 1) both;
  }
  .is-animating .t-digit[data-stagger="1"] {
    animation-delay: 70ms;
  }
  .is-animating .t-digit[data-stagger="2"] {
    animation-delay: 140ms;
  }
  .is-animating .t-digit[data-same] {
    animation: none;
  }
  @keyframes t-digit-pop-in {
    0% {
      transform: translateY(8px);
      opacity: 0;
      filter: blur(2px);
    }
    100% {
      transform: translateY(0);
      opacity: 1;
      filter: blur(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .t-digit-group .t-digit {
      animation: none !important;
    }
  }
</style>
