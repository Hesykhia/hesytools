<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, onToast, type Settings, type SystemInfo } from "$lib/api";
  import { toast, toasts } from "$lib/toast.svelte";
  import QuickClicker from "$lib/components/QuickClicker.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";

  type Page = "macros" | "watch" | "settings";
  const PAGES: { id: Page; label: string; icon: string }[] = [
    { id: "macros", label: "Macros", icon: "M4 6h16M4 12h10M4 18h7" },
    { id: "watch", label: "Screen Watch", icon: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zm10 3a3 3 0 100-6 3 3 0 000 6z" },
    { id: "settings", label: "Settings", icon: "M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6" },
  ];

  let page = $state<Page>("macros");
  let settings = $state<Settings | null>(null);
  let info = $state<SystemInfo | null>(null);
  let loadError = $state("");

  onMount(() => {
    api.getSettings().then((s) => (settings = s)).catch((e) => (loadError = errorText(e)));
    api.systemInfo().then((i) => (info = i));
    const un = onToast((m) => toast(m));
    return () => un.then((f) => f());
  });
</script>

<div class="app">
  <nav>
    <div class="brand">hesy<span>tools</span></div>
    {#each PAGES as p}
      <button class="nav" class:on={page === p.id} onclick={() => (page = p.id)}>
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={p.icon} /></svg>
        {p.label}
      </button>
    {/each}
    <div class="spacer"></div>
    {#if info?.elevated}<div class="badge">Admin</div>{/if}
    {#if settings?.panicHotkey}
      <div class="panic muted">Panic: <b>{settings.panicHotkey}</b></div>
    {/if}
  </nav>

  <main>
    {#if loadError}
      <div class="card">Couldn't load settings: {loadError}</div>
    {:else if !settings}
      <div class="muted">Loading…</div>
    {:else if page === "macros"}
      <div class="stack">
        <QuickClicker bind:settings />
        <section class="card soon">
          <h2>Macros</h2>
          <p class="muted">
            Record, edit and replay sequences of keys, clicks and waits, with loops, conditions and "wait until text appears on
            screen" steps. Coming in the next phase.
          </p>
        </section>
      </div>
    {:else if page === "watch"}
      <section class="card soon">
        <h2>Screen Watch</h2>
        <p class="muted">
          Pick a region of the screen and react to what's in it: text (OCR), numbers, pixel colours or images. It includes a
          live viewer of exactly what's being read. It's built on Windows' own OCR engine and only re-reads when the region
          actually changes, so it's cheap on CPU.
        </p>
      </section>
    {:else}
      <SettingsPanel bind:settings {info} />
    {/if}
  </main>

  <div class="toasts">
    {#each toasts as t (t.id)}
      <div class="toast" class:error={t.kind === "error"}>{t.text}</div>
    {/each}
  </div>
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
  }
  nav {
    background: var(--panel);
    border-right: 1px solid var(--line);
    padding: 18px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .brand {
    font-size: 19px;
    font-weight: 700;
    padding: 0 10px 18px;
    letter-spacing: -0.3px;
  }
  .brand span {
    color: var(--accent-2);
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    background: transparent;
    text-align: left;
    padding: 9px 10px;
    color: var(--muted);
  }
  .nav:hover {
    background: var(--panel-2) !important;
    color: var(--text);
  }
  .nav.on {
    background: var(--panel-2);
    color: var(--text);
  }
  .nav.on svg {
    color: var(--accent-2);
  }
  .spacer {
    flex: 1;
  }
  .badge {
    align-self: flex-start;
    margin: 0 10px 8px;
    font-size: 11px;
    color: var(--warn);
    border: 1px solid rgba(245, 165, 36, 0.4);
    border-radius: 999px;
    padding: 2px 8px;
  }
  .panic {
    font-size: 12px;
    padding: 0 10px;
  }
  main {
    overflow-y: auto;
    padding: 24px;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 900px;
  }
  main > :global(.stack),
  main > .card {
    max-width: 900px;
  }
  .soon p {
    margin: 8px 0 0;
    line-height: 1.5;
  }
  .toasts {
    position: fixed;
    right: 18px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 10;
  }
  .toast {
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-left: 3px solid var(--accent);
    border-radius: 8px;
    padding: 10px 14px;
    max-width: 380px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    font-size: 13px;
  }
  .toast.error {
    border-left-color: var(--bad);
  }
</style>
