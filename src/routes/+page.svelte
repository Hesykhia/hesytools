<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, onClickerState, onToast, type Settings, type SystemInfo } from "$lib/api";
  import { toast, toasts } from "$lib/toast.svelte";
  import Aurora from "$lib/components/ui/Aurora.svelte";
  import TitleBar from "$lib/components/ui/TitleBar.svelte";
  import Tabs from "$lib/components/ui/Tabs.svelte";
  import QuickClicker from "$lib/components/clicker/QuickClicker.svelte";
  import MacrosEmpty from "$lib/components/MacrosEmpty.svelte";
  import WatchPreview from "$lib/components/WatchPreview.svelte";
  import SettingsSheet from "$lib/components/SettingsSheet.svelte";

  type Page = "macros" | "watch";
  const ORDER: Page[] = ["macros", "watch"];

  let page = $state<Page>("macros");
  let direction = $state(1);
  let settings = $state<Settings | null>(null);
  let info = $state<SystemInfo | null>(null);
  let loadError = $state("");
  let running = $state(false);
  let settingsOpen = $state(false);

  function go(p: Page) {
    direction = ORDER.indexOf(p) > ORDER.indexOf(page) ? 1 : -1;
    page = p;
  }

  onMount(() => {
    api.getSettings().then((s) => (settings = s)).catch((e) => (loadError = errorText(e)));
    api.systemInfo().then((i) => (info = i));
    api.clickerStatus().then((s) => (running = s.running));
    // Keep the glow in sync even while the clicker panel isn't on screen.
    const unlisteners = [onToast((m) => toast(m)), onClickerState((r) => (running = r))];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  });
</script>

<Aurora active={running} />

<div class="app">
  <TitleBar>
    <div class="nav">
      <Tabs
        value={page}
        onchange={go}
        options={[
          { value: "macros", label: "Macros", dot: running },
          { value: "watch", label: "Screen Watch" },
        ]}
      />
      <button class="gear" aria-label="Settings" onclick={() => (settingsOpen = true)}>
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </div>
  </TitleBar>

  <main>
    {#if loadError}
      <div class="glass error">Couldn't load settings: {loadError}</div>
    {:else if settings}
      {#key page}
        <div class="page" style:--dir={direction}>
          {#if page === "macros"}
            <QuickClicker bind:settings bind:running />
            <MacrosEmpty />
          {:else}
            <WatchPreview />
          {/if}
          {#if settings.panicHotkey}
            <div class="panic">
              <span class="stop"></span>Panic key <kbd>{settings.panicHotkey}</kbd> stops everything
            </div>
          {/if}
        </div>
      {/key}
    {/if}
  </main>

</div>

{#if settings}
  <SettingsSheet bind:open={settingsOpen} bind:settings {info} />
{/if}

<div class="toasts">
  {#each toasts as t (t.id)}
    <div class="toast" class:error={t.kind === "error"}>{t.text}</div>
  {/each}
</div>

<style>
  .app {
    position: relative;
    z-index: 1;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .gear {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--text-3);
    border: 1px solid var(--stroke);
    background: rgba(0, 0, 0, 0.28);
    transition:
      color var(--duration-quick) var(--ease-smooth-out),
      transform 400ms var(--ease-smooth-out);
  }
  .gear:hover {
    color: var(--text);
    transform: rotate(45deg);
  }
  main {
    flex: 1;
    overflow-y: auto;
    padding: 8px 28px 22px;
  }
  .page {
    max-width: 940px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 26px;
    animation: page-in 250ms var(--ease-smooth-out) both;
  }
  .error {
    padding: 20px;
  }
  .panic {
    align-self: center;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    color: var(--text-3);
  }
  .panic kbd {
    font-family: inherit;
    font-weight: 650;
    font-size: 11px;
    color: var(--text-2);
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid var(--stroke);
  }
  .stop {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    background: var(--red);
    opacity: 0.8;
  }
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 22px;
    translate: -50% 0;
    z-index: 60;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .toast {
    padding: 10px 18px;
    border-radius: 999px;
    font-size: 13px;
    font-weight: 550;
    background: #1c1b26;
    border: 1px solid var(--stroke-strong);
    box-shadow: 0 16px 40px -10px rgba(0, 0, 0, 0.7);
    animation: toast-in 350ms var(--ease-smooth-out) both;
  }
  .toast.error {
    border-color: rgba(255, 92, 112, 0.5);
    color: #ffc2ca;
  }
  @keyframes page-in {
    from {
      opacity: 0;
      transform: translateX(calc(var(--dir) * 8px));
      filter: blur(3px);
    }
  }
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(16px) scale(0.97);
      filter: blur(2px);
    }
  }
</style>
