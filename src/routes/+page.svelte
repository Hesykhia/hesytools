<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    errorText,
    onClickerState,
    onMacrosChanged,
    onSettingsChanged,
    onToast,
    type Settings,
    type SystemInfo,
  } from "$lib/api";
  import { toast, toasts } from "$lib/toast.svelte";
  import Aurora from "$lib/components/ui/Aurora.svelte";
  import TitleBar from "$lib/components/ui/TitleBar.svelte";
  import Tabs from "$lib/components/ui/Tabs.svelte";
  import Autoclicker from "$lib/components/clicker/Autoclicker.svelte";
  import MacroLibrary from "$lib/components/macros/MacroLibrary.svelte";
  import MacroEditor from "$lib/components/macros/MacroEditor.svelte";
  import RecordingBar from "$lib/components/macros/RecordingBar.svelte";
  import RecordingCard from "$lib/components/macros/RecordingCard.svelte";
  import { activity, initActivity } from "$lib/activity.svelte";
  import WatchPage from "$lib/components/watch/WatchPage.svelte";
  import SettingsPage from "$lib/components/SettingsPage.svelte";
  import UpdateCard from "$lib/components/UpdateCard.svelte";
  import { checkForUpdates } from "$lib/update.svelte";
  import { initShutdown, remaining, shutdown } from "$lib/shutdown.svelte";

  type Page = "macros" | "watch" | "settings";
  const ORDER: Page[] = ["macros", "watch", "settings"];

  let page = $state<Page>("macros");
  let direction = $state(1);
  let settings = $state<Settings | null>(null);
  let info = $state<SystemInfo | null>(null);
  let loadError = $state("");
  let running = $state(false);
  /** Macro open in the editor, or null for the library. */
  let editing = $state<string | null>(null);
  let editingMacro = $derived(editing ? activity.macros.find((m) => m.id === editing) : undefined);
  let busy = $derived(running || activity.playingId !== null);
  let aurora = $derived<"idle" | "live" | "rec">(activity.recording ? "rec" : busy ? "live" : "idle");

  function openMacro(id: string | null) {
    direction = id ? 1 : -1;
    editing = id;
    page = "macros";
  }

  // A finished recording opens straight in the editor.
  $effect(() => {
    if (activity.openId) {
      openMacro(activity.openId);
      activity.openId = null;
    }
  });

  function go(p: Page) {
    direction = ORDER.indexOf(p) > ORDER.indexOf(page) ? 1 : -1;
    page = p;
  }

  onMount(() => {
    api
      .getSettings()
      .then((s) => {
        settings = s;
        // A quiet check a few seconds after launch, so startup stays snappy.
        if (s.autoCheckUpdates) setTimeout(() => checkForUpdates(), 4000);
      })
      .catch((e) => (loadError = errorText(e)));
    api.systemInfo().then((i) => (info = i));
    api.clickerStatus().then((s) => (running = s.running));
    // Keep the glow in sync even while the clicker panel isn't on screen.
    const unlisteners = [
      onToast((m) => toast(m)),
      onClickerState((r) => (running = r)),
      initActivity(),
      initShutdown(),
      // The backend changed something itself (a hotkey moved, panic turned
      // watches off): pick it up.
      onSettingsChanged((s) => {
        settings = s;
      }),
      onMacrosChanged((list) => {
        activity.macros = list;
      }),
    ];
    return () => unlisteners.forEach((u) => u.then((f) => f()));
  });
</script>

<Aurora mode={aurora} />

<div class="app">
  <TitleBar>
    {#if activity.recording}
      <RecordingBar hotkey={settings?.recordHotkey ?? null} />
    {:else}
    <div class="nav">
      <Tabs
        value={page}
        onchange={go}
        options={[
          { value: "macros", label: "Macro", dot: busy },
          { value: "watch", label: "Screen Watch" },
        ]}
      />

    </div>
    {/if}
    {#snippet right()}
      {#if shutdown.timer}
        <button class="shut-pill" onclick={() => go("settings")} title="Shutdown timer: click to change or cancel">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"
            ><path d="M12 3v8M6.4 6.4a8 8 0 1 0 11.2 0" /></svg
          >
          {remaining(shutdown.timer, shutdown.now)}
        </button>
      {/if}
      <button class="gear" class:on={page === "settings"} aria-label="Settings" title="Settings" onclick={() => go(page === "settings" ? "macros" : "settings")}>
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    {/snippet}
  </TitleBar>

  <main>
    {#if loadError}
      <div class="glass error">Couldn't load settings: {loadError}</div>
    {:else if settings}
      {#key `${page}:${editingMacro ? editing : ""}`}
        <div class="page" style:--dir={direction}>
          {#if page === "macros" && editingMacro}
            <MacroEditor source={editingMacro} onback={() => openMacro(null)} />
          {:else if page === "macros"}
            <div class="macros-layout">
              <div class="main-col">
                {#if activity.recording}<RecordingCard />{/if}
                <MacroLibrary bind:settings onopen={openMacro} />
              </div>
              <aside><Autoclicker bind:settings bind:running /></aside>
            </div>
          {:else if page === "watch"}
            <WatchPage bind:settings />
          {:else}
            <SettingsPage bind:settings {info} />
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

<UpdateCard />

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
  .shut-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    margin-right: 8px;
    border-radius: 999px;
    font-size: 12.5px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    color: #ffb3bd;
    background: rgba(255, 92, 112, 0.12);
    border: 1px solid rgba(255, 92, 112, 0.45);
  }
  .shut-pill:hover {
    background: rgba(255, 92, 112, 0.2);
  }
  .gear.on {
    color: var(--violet-2);
    border-color: rgba(139, 108, 255, 0.5);
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
  .macros-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 20px;
    align-items: start;
  }
  .main-col {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: 0;
  }
  aside {
    position: sticky;
    top: 0;
  }
  @media (max-width: 960px) {
    .macros-layout {
      grid-template-columns: 1fr;
    }
    aside {
      position: static;
    }
  }
  .page {
    max-width: 1500px;
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
