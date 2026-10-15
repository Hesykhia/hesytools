<script lang="ts">
  // "on the whole screen" / "in <app>": which pixels a watch looks at. An
  // app-only watch captures just that window (like OBS window capture), so
  // anything on top of it, e.g. a taskbar preview, can't trigger it.
  import { api, errorText, type Region, type WindowInfo, type WindowTarget } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import Popover from "../ui/Popover.svelte";

  let { window: target = $bindable(), region = $bindable() }: { window: WindowTarget | null | undefined; region: Region } = $props();

  let windows = $state<WindowInfo[]>([]);
  async function refresh() {
    try {
      windows = await api.listWindows();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  const appName = (exe: string) => exe.replace(/\.exe$/i, "");
  const has = () => region.w > 0 && region.h > 0;

  /** Keep the area on the same pixels when switching between screen and window coordinates. */
  async function choose(next: WindowInfo | null, close: () => void) {
    close();
    const prev = target ? await api.findWindow($state.snapshot(target)) : null;
    if (has()) {
      // Back to screen coordinates first (if the old window is around), then into the new window's.
      let x = region.x + (prev?.x ?? 0);
      let y = region.y + (prev?.y ?? 0);
      if (target && !prev) x = y = NaN;
      if (next) {
        x -= next.x;
        y -= next.y;
      }
      const inside = !next || (x >= 0 && y >= 0 && x + region.w <= next.w && y + region.h <= next.h);
      if (Number.isFinite(x) && inside) region = { ...region, x, y };
      else {
        region = { x: 0, y: 0, w: 0, h: 0 };
        toast("Select the area again, inside the window");
      }
    }
    target = next ? { exe: next.exe, title: next.title } : null;
  }
</script>

<Popover width={340}>
  {#snippet trigger({ toggle, open })}
    <button
      class="chip"
      class:open
      onclick={() => {
        if (!open) refresh();
        toggle();
      }}
      title="Look at the whole screen, or only one app's window">
      {target ? `in ${appName(target.exe)}` : "on the whole screen"}
    </button>
  {/snippet}
  {#snippet children({ close })}
    <button class="item" class:on={!target} onclick={() => choose(null, close)}>
      <b>The whole screen</b><small>Whatever is showing there, including windows on top</small>
    </button>
    <div class="eyebrow pad">Only this app</div>
    {#each windows as w (w.handle)}
      <button class="item" class:on={target?.exe === w.exe && target?.title === w.title} onclick={() => choose(w, close)}>
        <b>{appName(w.exe)}</b><small>{w.title}</small>
      </button>
    {:else}
      <p class="none">Looking for open windows…</p>
    {/each}
    <p class="note">Only that window is captured, so nothing on top of it counts. It can't see a minimized window.</p>
  {/snippet}
</Popover>

<style>
  .chip {
    font-weight: 600;
    font-size: 13px;
    padding: 2px 10px;
    border-radius: 8px;
    color: var(--text-2);
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--stroke);
  }
  .chip:hover,
  .chip.open {
    color: var(--text);
    background: var(--glass-hover);
  }
  .item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    border-radius: 9px;
  }
  .item:hover {
    background: var(--glass-hover);
  }
  .item b {
    display: block;
    font-weight: 600;
    font-size: 13.5px;
  }
  .item small {
    display: block;
    color: var(--text-3);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .item.on b {
    color: var(--violet-2);
  }
  .pad {
    padding: 10px 10px 4px;
  }
  .none,
  .note {
    font-size: 12px;
    color: var(--text-3);
    padding: 4px 10px 6px;
  }
  .note {
    border-top: 1px solid var(--stroke);
    margin-top: 6px;
    padding-top: 8px;
  }
</style>
