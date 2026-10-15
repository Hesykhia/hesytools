<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Snippet } from "svelte";

  let { children }: { children?: Snippet } = $props();
  const win = getCurrentWindow();
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <img class="mark" src="/logo.png" alt="" aria-hidden="true" />
    <span class="name" data-tauri-drag-region>hesytools</span>
  </div>

  <div class="center">{@render children?.()}</div>

  <div class="controls">
    <button aria-label="Minimize" onclick={() => win.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 5h8" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button aria-label="Maximize" onclick={() => win.toggleMaximize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
    <button class="close" aria-label="Close" onclick={() => win.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" stroke="currentColor" stroke-width="1.2" /></svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    position: relative;
    z-index: 5;
    height: 52px;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 8px 0 18px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .mark {
    width: 22px;
    height: 22px;
    filter: drop-shadow(0 0 10px rgba(139, 108, 255, 0.5));
  }
  .name {
    font-weight: 650;
    letter-spacing: -0.02em;
    font-size: 15px;
  }
  .center {
    display: flex;
    justify-content: center;
  }
  .controls {
    justify-self: end;
    display: flex;
    gap: 2px;
  }
  .controls button {
    width: 38px;
    height: 30px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: var(--text-3);
    transition:
      background var(--duration-quick) var(--ease-smooth-out),
      color var(--duration-quick) var(--ease-smooth-out);
  }
  .controls button:hover {
    background: var(--glass-hover);
    color: var(--text);
  }
  .controls .close:hover {
    background: rgba(255, 92, 112, 0.9);
    color: white;
  }
</style>
