<script lang="ts">
  // "anywhere" / "only in <app>": input only goes out while that app is in
  // front and under the pointer, so alt-tabbing or the taskbar is safe.
  import { api, errorText, type WindowInfo, type WindowTarget } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import Popover from "./ui/Popover.svelte";

  let {
    value,
    onchange,
    what = "clicks",
  }: {
    value: WindowTarget | null | undefined;
    onchange: (v: WindowTarget | null) => void;
    /** "clicks" or "keys and clicks", for the explanation. */
    what?: string;
  } = $props();

  let windows = $state<WindowInfo[]>([]);
  async function refresh() {
    try {
      windows = await api.listWindows();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }
  const appName = (exe: string) => exe.replace(/\.exe$/i, "");
  // One entry per program: the lock is by program, not by window.
  let programs = $derived([...new Map(windows.map((w) => [w.exe.toLowerCase(), w])).values()]);
</script>

<Popover width={320}>
  {#snippet trigger({ toggle, open })}
    <button
      class="chip"
      class:open
      class:set={!!value}
      onclick={() => {
        if (!open) refresh();
        toggle();
      }}>{value ? `only in ${appName(value.exe)}` : "in any app"}</button
    >
  {/snippet}
  {#snippet children({ close })}
    <button class="item" class:on={!value} onclick={() => (onchange(null), close())}>
      <b>Any app</b><small>Wherever you are</small>
    </button>
    <div class="eyebrow pad">Only in</div>
    {#each programs as w (w.exe)}
      <button
        class="item"
        class:on={value?.exe.toLowerCase() === w.exe.toLowerCase()}
        onclick={() => (onchange({ exe: w.exe, title: w.title }), close())}
      >
        <b>{appName(w.exe)}</b><small>{w.title}</small>
      </button>
    {:else}
      <p class="none">Looking for open apps…</p>
    {/each}
    <p class="note">
      No {what} go out unless that app is in front and under the pointer. Tab away or hover the taskbar and it waits, then
      carries on when you're back.
    </p>
  {/snippet}
</Popover>

<style>
  .chip {
    font-weight: 650;
    padding: 2px 10px;
    border-radius: 8px;
    color: var(--text-2);
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--stroke);
  }
  .chip.set {
    color: var(--text);
    background: rgba(45, 212, 191, 0.14);
    box-shadow: inset 0 0 0 1px rgba(45, 212, 191, 0.45);
  }
  .chip:hover,
  .chip.open {
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
