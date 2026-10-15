<script lang="ts">
  import { api, errorText } from "$lib/api";
  import { toast } from "$lib/toast.svelte";

  let {
    value,
    onchange,
    placeholder = "Not set",
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
  <button class="key" class:capturing onclick={capture} title="Click, then press a key or mouse button. Esc cancels.">
    {#if capturing}
      Press a key…
    {:else if value}
      {#each value.split("+") as part, i}
        {#if i > 0}<span class="plus">+</span>{/if}<kbd>{part}</kbd>
      {/each}
    {:else}
      <span class="muted">{placeholder}</span>
    {/if}
  </button>
  {#if clearable && value && !capturing}
    <button class="clear" title="Clear" onclick={() => onchange(null)}>×</button>
  {/if}
</span>

<style>
  .hk {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .key {
    min-width: 110px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
  }
  .key.capturing {
    border-color: var(--accent);
    color: var(--accent-2);
    animation: pulse 1.2s ease-in-out infinite;
  }
  kbd {
    font-family: inherit;
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    padding: 1px 6px;
  }
  .plus {
    color: var(--muted);
    font-size: 11px;
  }
  .clear {
    padding: 2px 8px;
    color: var(--muted);
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 3px rgba(124, 92, 255, 0.25);
    }
  }
</style>
