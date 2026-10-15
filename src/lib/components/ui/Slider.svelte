<script lang="ts">
  let {
    value,
    min = 0,
    max = 100,
    step = 1,
    onchange,
  }: { value: number; min?: number; max?: number; step?: number; onchange: (v: number) => void } = $props();

  let pct = $derived(((Math.min(Math.max(value, min), max) - min) / (max - min)) * 100);
</script>

<input
  class="slider"
  type="range"
  {min}
  {max}
  {step}
  {value}
  style:--pct="{pct}%"
  oninput={(e) => onchange(Number(e.currentTarget.value))}
/>

<style>
  .slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 22px;
    background: transparent;
    cursor: pointer;
  }
  .slider::-webkit-slider-runnable-track {
    height: 6px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--violet) 0%, var(--violet-2) var(--pct), rgba(255, 255, 255, 0.1) var(--pct));
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    margin-top: -6px;
    border-radius: 50%;
    background: white;
    box-shadow:
      0 0 0 4px rgba(139, 108, 255, 0.25),
      0 2px 8px rgba(0, 0, 0, 0.4);
    transition: box-shadow var(--duration-quick) var(--ease-smooth-out);
  }
  .slider:hover::-webkit-slider-thumb {
    box-shadow:
      0 0 0 6px rgba(139, 108, 255, 0.3),
      0 2px 8px rgba(0, 0, 0, 0.4);
  }
</style>
