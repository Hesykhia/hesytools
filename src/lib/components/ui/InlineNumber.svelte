<script lang="ts">
  // A number field that sits inside a sentence and sizes itself to its value.
  let {
    value,
    onchange,
    min = 0,
    max = 1e9,
    width,
  }: {
    value: number;
    onchange: (v: number) => void;
    min?: number;
    max?: number;
    /** Fixed width in ch; otherwise grows with the value. */
    width?: number;
  } = $props();

  let text = $state("");
  let focused = $state(false);
  $effect(() => {
    if (!focused) text = String(Math.round(value * 1000) / 1000);
  });

  function commit() {
    const n = Number(text);
    if (text.trim() !== "" && Number.isFinite(n)) onchange(Math.min(max, Math.max(min, n)));
    else text = String(value);
  }
</script>

<input
  class="num"
  type="text"
  inputmode="decimal"
  bind:value={text}
  style:width="{(width ?? Math.max(2, text.length)) + 1.4}ch"
  onfocus={(e) => {
    focused = true;
    e.currentTarget.select();
  }}
  onblur={() => {
    focused = false;
    commit();
  }}
  onkeydown={(e) => {
    if (e.key === "Enter") e.currentTarget.blur();
    if (e.key === "Escape") {
      text = String(value);
      e.currentTarget.blur();
    }
  }}
/>

<style>
  .num {
    font: inherit;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    text-align: center;
    color: var(--text);
    padding: 1px 4px;
    border-radius: 8px;
    border: 1px solid transparent;
    background: rgba(255, 255, 255, 0.07);
    transition:
      background var(--duration-quick) var(--ease-smooth-out),
      border-color var(--duration-quick) var(--ease-smooth-out),
      box-shadow var(--duration-quick) var(--ease-smooth-out);
  }
  .num:hover {
    background: rgba(255, 255, 255, 0.11);
  }
  .num:focus {
    border-color: rgba(139, 108, 255, 0.6);
    box-shadow: 0 0 0 3px rgba(139, 108, 255, 0.15);
    background: rgba(0, 0, 0, 0.3);
  }
</style>
