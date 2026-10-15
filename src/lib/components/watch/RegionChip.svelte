<script lang="ts">
  import { api, errorText, type Region, type WindowTarget } from "$lib/api";
  import { toast } from "$lib/toast.svelte";

  let { region = $bindable(), window = null }: { region: Region; window?: WindowTarget | null } = $props();
  let picking = $state(false);
  let has = $derived(region.w > 0 && region.h > 0);

  async function pick() {
    picking = true;
    try {
      const r = await api.pickRegion();
      if (!r) return;
      if (!window) {
        region = r;
        return;
      }
      // App-only watch: store the area relative to that window.
      const w = await api.findWindow($state.snapshot(window));
      if (!w) return toast(`${window.exe} isn't open`, "error");
      const x = r.x - w.x;
      const y = r.y - w.y;
      if (x < 0 || y < 0 || x + r.w > w.w || y + r.h > w.h) return toast("Select an area inside the window", "error");
      region = { x, y, w: r.w, h: r.h };
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      picking = false;
    }
  }
</script>

<button class="region" class:empty={!has} onclick={pick} title="Drag a box around the part of the screen to watch">
  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"
    ><path d="M4 8V5a1 1 0 0 1 1-1h3M16 4h3a1 1 0 0 1 1 1v3M20 16v3a1 1 0 0 1-1 1h-3M8 20H5a1 1 0 0 1-1-1v-3" /></svg
  >
  {#if picking}
    Selecting…
  {:else if has}
    {region.w}×{region.h} area
  {:else}
    Select area
  {/if}
</button>

<style>
  .region {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 650;
    font-size: 13px;
    padding: 3px 10px;
    border-radius: 8px;
    color: var(--cyan);
    background: rgba(56, 214, 255, 0.1);
    border: 1px solid rgba(56, 214, 255, 0.35);
    transition: background var(--duration-quick) var(--ease-smooth-out);
  }
  .region:hover {
    background: rgba(56, 214, 255, 0.2);
  }
  .region.empty {
    border-style: dashed;
  }
</style>
