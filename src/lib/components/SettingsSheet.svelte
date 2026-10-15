<script lang="ts">
  import { untrack } from "svelte";
  import { api, errorText, type Settings, type SystemInfo, type WebhookSettings } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "./HotkeyButton.svelte";
  import Toggle from "./ui/Toggle.svelte";

  let {
    open = $bindable(false),
    settings = $bindable(),
    info,
  }: { open?: boolean; settings: Settings; info: SystemInfo | null } = $props();

  // svelte-ignore state_referenced_locally
  let webhook = $state<WebhookSettings>($state.snapshot(settings.webhook));
  let dirty = $derived(JSON.stringify(webhook) !== JSON.stringify(settings.webhook));
  let testing = $state(false);
  let closing = $state(false);
  let shown = $state(false);

  $effect(() => {
    if (open) {
      webhook = untrack(() => $state.snapshot(settings.webhook));
      shown = false;
      requestAnimationFrame(() => (shown = true));
    }
  });

  function close() {
    closing = true;
    setTimeout(() => {
      open = false;
      closing = false;
    }, 150);
  }

  async function saveWebhook() {
    try {
      const w = $state.snapshot(webhook);
      await api.setWebhook(w);
      settings.webhook = w;
      toast("Discord settings saved");
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function testWebhook() {
    testing = true;
    try {
      await api.testWebhook($state.snapshot(webhook));
      toast("Test sent. Check your Discord channel.");
    } catch (e) {
      toast(errorText(e), "error", 6000);
    } finally {
      testing = false;
    }
  }

  async function setPanic(v: string | null) {
    try {
      settings = await api.setHotkey("panic", v);
    } catch (e) {
      toast(errorText(e), "error");
    }
  }
</script>

<svelte:window onkeydown={(e) => open && e.key === "Escape" && close()} />

{#if open}
  <div class="scrim" class:shown={shown && !closing} onpointerdown={(e) => e.target === e.currentTarget && close()} role="presentation">
    <div class="sheet glass" class:shown={shown && !closing} class:closing role="dialog" aria-label="Settings">
      <header>
        <h2>Settings</h2>
        <button class="x" aria-label="Close" onclick={close}>
          <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 2l8 8M10 2l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" /></svg>
        </button>
      </header>

      <div class="group">
        <div class="row">
          <span class="icon red">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><rect x="6" y="6" width="12" height="12" rx="2" /></svg>
          </span>
          <div class="text">
            <b>Panic key</b>
            <small>Stops the clicker and every macro instantly, from anywhere</small>
          </div>
          <HotkeyButton value={settings.panicHotkey} onchange={setPanic} />
        </div>
      </div>

      <div class="group">
        <div class="row">
          <span class="icon discord">
            <svg width="17" height="17" viewBox="0 0 24 24" fill="currentColor"><path d="M19.3 5.3A16.6 16.6 0 0 0 15.2 4l-.5 1a15 15 0 0 0-5.4 0l-.5-1a16.5 16.5 0 0 0-4.1 1.3C2.1 9.2 1.4 13 1.7 16.8a16.7 16.7 0 0 0 5 2.5l1.1-1.7a10.7 10.7 0 0 1-1.7-.8l.4-.3a11.9 11.9 0 0 0 11 0l.4.3-1.7.8 1.1 1.7a16.6 16.6 0 0 0 5-2.5c.4-4.4-.7-8.2-3-11.5zM8.7 14.5c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2zm6.6 0c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2z" /></svg>
          </span>
          <div class="text">
            <b>Discord alerts</b>
            <small>Get a message when the clicker finishes. Macro and screen alerts come later.</small>
          </div>
        </div>
        <div class="fields">
          <input type="url" placeholder="Paste webhook URL: https://discord.com/api/webhooks/…" bind:value={webhook.url} spellcheck="false" />
          <input type="text" placeholder="Your Discord user ID to get pinged (optional)" bind:value={webhook.mentionUserId} />
          <div class="row tight">
            <div class="text"><small class="plain">Alert when the Quick Clicker finishes a count or time limit</small></div>
            <Toggle checked={webhook.notifyClickerFinished} onchange={(v) => (webhook.notifyClickerFinished = v)} label="Notify on finish" />
          </div>
          <div class="actions">
            <span class="how">Channel settings → Integrations → Webhooks → Copy URL</span>
            <button class="btn" disabled={!webhook.url.trim() || testing} onclick={testWebhook}>{testing ? "Sending…" : "Send test"}</button>
            <button class="btn primary" disabled={!dirty} onclick={saveWebhook}>Save</button>
          </div>
        </div>
      </div>

      <div class="group">
        <div class="row">
          <span class="icon amber">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6z" /></svg>
          </span>
          <div class="text">
            <b>Admin mode {#if info?.elevated}<span class="on">On</span>{/if}</b>
            <small>Only needed for apps that run as administrator. Roblox and Minecraft don't.</small>
          </div>
          {#if !info?.elevated}
            <button class="btn" onclick={() => api.relaunchAsAdmin().catch((e) => toast(errorText(e), "error"))}>Restart as admin</button>
          {/if}
        </div>
      </div>

      <p class="ver">hesytools {info?.version ?? ""}</p>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgba(4, 4, 8, 0.7);
    opacity: 0;
    transition: opacity 250ms var(--ease-smooth-out);
  }
  .scrim.shown {
    opacity: 1;
  }
  .sheet {
    width: 600px;
    max-height: calc(100vh - 80px);
    overflow-y: auto;
    padding: 22px;
    background: rgba(18, 17, 26, 0.92);
    border-radius: 26px;
    transform: scale(0.96);
    opacity: 0;
    transition:
      transform 250ms var(--ease-smooth-out),
      opacity 250ms var(--ease-smooth-out);
  }
  .sheet.shown {
    transform: scale(1);
    opacity: 1;
  }
  .sheet.closing {
    transform: scale(0.96);
    opacity: 0;
    transition-duration: 150ms;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 18px;
    padding: 0 4px;
  }
  h2 {
    font-size: 20px;
    letter-spacing: -0.02em;
  }
  .x {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--text-3);
    background: var(--glass);
  }
  .x:hover {
    color: var(--text);
    background: var(--glass-hover);
  }
  .group {
    padding: 16px;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--stroke);
    margin-bottom: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .row.tight {
    gap: 10px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .text b {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .text small {
    display: block;
    color: var(--text-3);
    font-size: 12.5px;
    margin-top: 2px;
  }
  .text small.plain {
    color: var(--text-2);
    font-size: 13px;
  }
  .on {
    font-size: 11px;
    color: var(--mint);
    background: rgba(61, 242, 176, 0.12);
    padding: 1px 8px;
    border-radius: 99px;
  }
  .icon {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 11px;
    display: grid;
    place-items: center;
  }
  .icon.red {
    color: var(--red);
    background: rgba(255, 92, 112, 0.14);
  }
  .icon.discord {
    color: #8e97ff;
    background: rgba(88, 101, 242, 0.18);
  }
  .icon.amber {
    color: var(--amber);
    background: rgba(255, 191, 92, 0.13);
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 14px;
    padding-left: 50px;
  }
  .fields input {
    width: 100%;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .how {
    flex: 1;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .ver {
    text-align: center;
    font-size: 12px;
    color: var(--text-3);
    margin-top: 12px;
  }
  @media (prefers-reduced-motion: reduce) {
    .sheet,
    .scrim {
      transition: none !important;
    }
  }
</style>
