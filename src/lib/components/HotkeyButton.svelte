<script lang="ts">
  import { api, errorText } from "$lib/api";
  import { toast } from "$lib/toast.svelte";

  let {
    value,
    onchange,
    placeholder = "Set hotkey",
    clearable = true,
  }: {
    value: string | null;
    /** Called with the captured combo (or null when cleared). */
    onchange: (v: string | null) => void | Promise<void>;
    placeholder?: string;
    clearable?: boolean;
  } = $props();

  let capturing = $state(false);

  async function capture() {
    if (capturing) {
      await api.cancelCapture();
      return;
    }
    capturing = true;
    try {
      const got = await api.captureHotkey();
      if (got) await onchange(got);
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      capturing = false;
    }
  }
</script>

<span class="hk">
  <button class="key" class:capturing onclick={capture} title="Click, then press any key or mouse button. Esc cancels.">
    {#if capturing}
      <span class="listening">Press any key…</span>
    {:else if value}
      {#each value.split("+") as part}
        <kbd>{part}</kbd>
      {/each}
    {:else}
      <span class="placeholder">{placeholder}</span>
    {/if}
  </button>
  {#if clearable && value && !capturing}
    <button class="clear" title="Remove hotkey" aria-label="Remove hotkey" onclick={() => onchange(null)}>
      <svg width="8" height="8" viewBox="0 0 8 8"><path d="M1 1l6 6M7 1L1 7" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
    </button>
  {/if}
</span>

<style>
  .hk {
    position: relative;
    display: inline-flex;
    align-items: center;
  }
  .key {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-height: 30px;
    padding: 3px;
    border-radius: 10px;
    transition: background var(--duration-quick) var(--ease-smooth-out);
  }
  .key:hover {
    background: var(--glass-hover);
  }
  kbd {
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 600;
    min-width: 26px;
    text-align: center;
    padding: 3px 8px;
    border-radius: 7px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.14), rgba(255, 255, 255, 0.06));
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow:
      inset 0 -2px 0 rgba(0, 0, 0, 0.35),
      0 2px 6px -2px rgba(0, 0, 0, 0.5);
  }
  .placeholder {
    color: var(--text-3);
    padding: 0 8px;
    font-size: 13px;
  }
  .listening {
    padding: 3px 10px;
    font-size: 12.5px;
    font-weight: 600;
    border-radius: 7px;
    color: var(--violet-2);
    background: rgba(139, 108, 255, 0.12);
    box-shadow: 0 0 0 1px rgba(139, 108, 255, 0.5);
    animation: listen 1.4s ease-in-out infinite;
  }
  .clear {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #2a2838;
    border: 1px solid var(--stroke-strong);
    display: grid;
    place-items: center;
    color: var(--text-3);
    opacity: 0;
    transition: opacity var(--duration-quick) var(--ease-smooth-out);
  }
  .hk:hover .clear {
    opacity: 1;
  }
  .clear:hover {
    background: #3a374d;
    color: var(--text);
  }
  @keyframes listen {
    50% {
      box-shadow:
        0 0 0 1px rgba(139, 108, 255, 0.5),
        0 0 0 5px rgba(139, 108, 255, 0.12);
    }
  }
</style>
