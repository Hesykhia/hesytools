<script lang="ts">
  // A colour swatch: click it to choose a colour, or use the eyedropper to
  // pick one straight off the screen (works over games and any monitor).
  import { api, errorText } from "$lib/api";
  import { toast } from "$lib/toast.svelte";

  let { rgb = $bindable() }: { rgb: [number, number, number] } = $props();

  const hex = (c: number[]) => "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("");
  const parse = (h: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];

  let picking = $state(false);
  async function eyedrop() {
    picking = true;
    try {
      const c = await api.pickColor();
      if (c) rgb = c;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      picking = false;
    }
  }
</script>

<span class="pick">
  <label class="swatch" style:background={hex(rgb)} title="Choose a colour ({hex(rgb)})">
    <input type="color" value={hex(rgb)} oninput={(e) => (rgb = parse(e.currentTarget.value))} />
  </label>
  <span class="hex">{hex(rgb)}</span>
  <button class="drop" class:busy={picking} onclick={eyedrop} title="Pick a colour from anywhere on screen (Esc cancels)">
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"
      ><path d="M14 4l6 6M17 7l-9.5 9.5a2 2 0 0 1-1 .55L3 18l.95-3.5a2 2 0 0 1 .55-1L14 4" /><path d="M11 7l6 6" /></svg
    >
    {picking ? "click a spot…" : "pick from screen"}
  </button>
</span>

<style>
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .swatch {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 7px;
    cursor: pointer;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.3),
      0 0 12px rgba(0, 0, 0, 0.4);
  }
  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .hex {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .drop {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    padding: 3px 9px;
    border-radius: 99px;
    border: 1px solid var(--stroke);
  }
  .drop:hover {
    color: var(--text);
    border-color: var(--stroke-strong);
  }
  .drop.busy {
    color: var(--cyan);
    border-color: rgba(56, 214, 255, 0.5);
  }
</style>
