<script lang="ts">
  import { api, errorText, type Settings, type SystemInfo, type WebhookSettings } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "./HotkeyButton.svelte";

  let { settings = $bindable(), info }: { settings: Settings; info: SystemInfo | null } = $props();

  // svelte-ignore state_referenced_locally
  let webhook = $state<WebhookSettings>($state.snapshot(settings.webhook));
  let dirty = $derived(JSON.stringify(webhook) !== JSON.stringify(settings.webhook));
  let testing = $state(false);

  async function saveWebhook() {
    try {
      const w = $state.snapshot(webhook);
      await api.setWebhook(w);
      settings.webhook = w;
      toast("Webhook saved");
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  async function testWebhook() {
    testing = true;
    try {
      await api.testWebhook($state.snapshot(webhook));
      toast("Test message sent. Check your Discord channel.");
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

  async function relaunch() {
    try {
      await api.relaunchAsAdmin();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }
</script>

<div class="stack">
  <section class="card">
    <h3>Safety</h3>
    <div class="line">
      <div>
        <div>Panic hotkey</div>
        <div class="muted small">Instantly stops the clicker and any running macro, from anywhere.</div>
      </div>
      <HotkeyButton value={settings.panicHotkey} onchange={setPanic} />
    </div>
  </section>

  <section class="card">
    <h3>Discord notifications</h3>
    <p class="muted small">
      Get a message when things happen, like the clicker finishing (macro and screen-watch alerts come later). In Discord:
      channel settings → Integrations → Webhooks → New Webhook → Copy Webhook URL.
    </p>
    <div class="field">
      <label for="wh-url">Webhook URL</label>
      <input id="wh-url" type="url" placeholder="https://discord.com/api/webhooks/…" bind:value={webhook.url} spellcheck="false" />
    </div>
    <div class="field">
      <label for="wh-user">Ping user ID <span class="muted">(optional)</span></label>
      <input id="wh-user" type="text" placeholder="e.g. 123456789012345678" bind:value={webhook.mentionUserId} />
    </div>
    <label class="check">
      <input type="checkbox" bind:checked={webhook.notifyClickerFinished} />
      Notify when Quick Clicker finishes its click count or time limit
    </label>
    <div class="row actions">
      <button class="primary" disabled={!dirty} onclick={saveWebhook}>Save</button>
      <button disabled={!webhook.url.trim() || testing} onclick={testWebhook}>{testing ? "Sending…" : "Send test"}</button>
    </div>
  </section>

  <section class="card">
    <h3>Admin mode</h3>
    <div class="line">
      <div>
        <div>
          Currently running as
          {#if info?.elevated}<b class="good">administrator</b>{:else}<b>normal user</b>{/if}
        </div>
        <div class="muted small">
          Windows doesn't let normal apps send input to apps running as administrator. You only need this if a game
          or app ignores hesytools <i>and</i> runs as admin. Roblox and Minecraft don't.
        </div>
      </div>
      {#if !info?.elevated}
        <button onclick={relaunch}>Restart as admin</button>
      {/if}
    </div>
  </section>

  <p class="muted small about">hesytools {info?.version ?? ""}</p>
</div>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  h3 {
    margin-bottom: 12px;
    font-size: 15px;
  }
  .small {
    font-size: 12px;
    margin: 4px 0 0;
  }
  .line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 12px;
  }
  .field label {
    font-size: 13px;
  }
  .field input {
    width: 100%;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 14px;
  }
  .actions {
    margin-top: 16px;
  }
  .good {
    color: var(--good);
  }
  .about {
    text-align: center;
  }
</style>
