<script lang="ts">
  // Everything in one place: every hotkey, recording, startup, tray,
  // notifications, admin mode and the disclaimer.
  import { onMount } from "svelte";
  import { api, errorText, type HotkeyMode, type Macro, type Settings, type SystemInfo, type WebhookSettings } from "$lib/api";
  import { activity, upsertMacro } from "$lib/activity.svelte";
  import { toast } from "$lib/toast.svelte";
  import HotkeyButton from "./HotkeyButton.svelte";
  import Tabs from "./ui/Tabs.svelte";
  import Toggle from "./ui/Toggle.svelte";
  import { checkForUpdates, update } from "$lib/update.svelte";
  import { cancelShutdown, remaining, shutdown, startShutdown } from "$lib/shutdown.svelte";
  import InlineNumber from "./ui/InlineNumber.svelte";
  import { dev, setDev } from "$lib/dev.svelte";
  import { SCALE_LEVELS, scaleLevel } from "$lib/scale";
  import type { ScalePage } from "$lib/api";

  let { settings = $bindable(), info }: { settings: Settings; info: SystemInfo | null } = $props();

  async function run(task: () => Promise<unknown>) {
    try {
      await task();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  // ── Hotkeys ──────────────────────────────────────────────────────────────
  const setGlobal = (id: "clicker" | "panic" | "record", v: string | null) =>
    run(async () => (settings = await api.setHotkey(id, v)));
  const setMode = (mode: HotkeyMode) =>
    run(async () => {
      await api.setClickerMode(mode);
      settings.clickerMode = mode;
    });
  const setMacroHotkey = (m: Macro, hotkey: string | null) =>
    run(async () => upsertMacro(await api.saveMacro({ ...$state.snapshot(m), hotkey })));

  // ── Startup ──────────────────────────────────────────────────────────────
  let startup = $state(false);
  onMount(() => {
    api.startupEnabled().then((v) => (startup = v));
  });
  const setStartup = (on: boolean) =>
    run(async () => {
      await api.setStartupEnabled(on);
      startup = on;
    });
  const setStartupMacro = (id: string) =>
    run(async () => {
      const v = id || null;
      await api.setStartupMacro(v);
      settings.startupMacro = v;
    });

  // ── Shutdown timer ───────────────────────────────────────────────────────
  let shutMinutes = $state(30);
  let shutRestart = $state(false);
  const SHUT_PRESETS = [5, 15, 30, 60, 120];

  // ── GUI scale ────────────────────────────────────────────────────────────
  let scalePages = $derived<{ page: ScalePage; name: string }[]>([
    { page: "macros", name: "Macro" },
    { page: "watch", name: "Screen Watch" },
    ...(dev.unlocked ? [{ page: "dev" as ScalePage, name: "Dev" }] : []),
    { page: "settings", name: "Settings" },
  ]);
  const setScale = (page: ScalePage, level: number) => run(async () => (settings = await api.setUiScale(page, level)));

  // ── Dev access ───────────────────────────────────────────────────────────
  let devCode = $state("");
  const unlockDev = () =>
    run(async () => {
      setDev(await api.setDevCode(devCode));
      devCode = "";
      toast(dev.unlocked ? `Dev tab unlocked for ${dev.name}` : "Code removed");
    });
  const lockDev = () =>
    run(async () => {
      setDev(await api.setDevCode(""));
    });

  // ── Discord ──────────────────────────────────────────────────────────────
  // svelte-ignore state_referenced_locally
  let webhook = $state<WebhookSettings>($state.snapshot(settings.webhook));
  let dirty = $derived(JSON.stringify(webhook) !== JSON.stringify(settings.webhook));
  let testing = $state(false);
  const saveWebhook = () =>
    run(async () => {
      const w = $state.snapshot(webhook);
      await api.setWebhook(w);
      settings.webhook = w;
      toast("Discord settings saved");
    });
  async function testWebhook() {
    testing = true;
    await run(async () => {
      await api.testWebhook($state.snapshot(webhook));
      toast("Test sent. Check your Discord channel.");
    });
    testing = false;
  }
</script>

<div class="settings">
  <header class="head">
    <h2>Settings</h2>
    <p>Everything in one place. Changes save as you make them.</p>
  </header>

  <div class="cols">
    <div class="col">
      <section class="group glass">
        <h3>Hotkeys</h3>
        <p class="sub">They work from anywhere, even while a game is focused.</p>
        <div class="row">
          <div class="text"><b>Autoclicker</b><small>Start and stop clicking</small></div>
          <Tabs
            size="s"
            value={settings.clickerMode}
            onchange={setMode}
            options={[
              { value: "toggle", label: "Toggle" },
              { value: "hold", label: "Hold" },
            ]}
          />
          <HotkeyButton value={settings.clickerHotkey} onchange={(v) => setGlobal("clicker", v)} />
        </div>
        <div class="row">
          <div class="text"><b>Record a macro</b><small>Start and stop recording</small></div>
          <HotkeyButton value={settings.recordHotkey} onchange={(v) => setGlobal("record", v)} />
        </div>
        <div class="row">
          <div class="text"><b class="red">Panic: stop everything</b><small>Clicker, macros, recording and screen watches</small></div>
          <HotkeyButton value={settings.panicHotkey} onchange={(v) => setGlobal("panic", v)} />
        </div>
        {#if activity.macros.length > 0}
          <span class="label">Macro hotkeys</span>
          {#each activity.macros as m (m.id)}
            <div class="row">
              <div class="text"><b>{m.name}</b><small>Start and stop this macro</small></div>
              <HotkeyButton value={m.hotkey} onchange={(v) => setMacroHotkey(m, v)} placeholder="No hotkey" />
            </div>
          {/each}
        {/if}
      </section>

      <section class="group glass">
        <h3>Recording</h3>
        <div class="row">
          <div class="text">
            <b>Red outline while recording</b>
            <small>A thin border around every screen. Clicks go straight through it.</small>
          </div>
          <Toggle
            checked={settings.recordingOverlay}
            onchange={(v) =>
              run(async () => {
                await api.setRecordingOverlay(v);
                settings.recordingOverlay = v;
              })}
            label="Red outline while recording"
          />
        </div>
        <div class="row">
          <div class="text">
            <b>Record mouse movement</b>
            <small>Off: the mouse jumps straight to each click (fastest). On: replays how you moved it.</small>
          </div>
          <Toggle
            checked={settings.record.recordMoves}
            onchange={(v) =>
              run(async () => {
                const options = { ...settings.record, recordMoves: v };
                await api.setRecordOptions(options);
                settings.record = options;
              })}
            label="Record mouse movement"
          />
        </div>
      </section>

      <section class="group glass">
        <h3>GUI scale</h3>
        <p class="sub">How big each page is drawn, like Minecraft's GUI Scale. 3 is normal, 1 is smallest, 5 is biggest.</p>
        {#each scalePages as s (s.page)}
          <div class="row">
            <div class="text"><b>{s.name}</b></div>
            <Tabs
              size="s"
              value={String(scaleLevel(settings, s.page))}
              onchange={(v) => setScale(s.page, Number(v))}
              options={SCALE_LEVELS.map((n) => ({ value: String(n), label: String(n) }))}
            />
          </div>
        {/each}
      </section>
    </div>

    <div class="col">
      <section class="group glass" class:armed={shutdown.timer}>
        <h3>Shut down PC</h3>
        {#if shutdown.timer}
          <div class="armed-row">
            <div class="text">
              <b class="countdown">{remaining(shutdown.timer, shutdown.now)}</b>
              <small>until your PC {shutdown.timer.restart ? "restarts" : "shuts down"}</small>
            </div>
            <button class="btn primary" onclick={cancelShutdown}>Cancel</button>
          </div>
        {:else}
          <p class="sub">Leave something running and let hesytools turn the PC off when it's done.</p>
          <div class="shut-line">
            <Tabs
              size="s"
              value={shutRestart ? "restart" : "shutdown"}
              onchange={(v) => (shutRestart = v === "restart")}
              options={[
                { value: "shutdown", label: "Shut down" },
                { value: "restart", label: "Restart" },
              ]}
            />
            <span class="soft">in</span>
            <InlineNumber value={shutMinutes} min={1} max={14400} onchange={(v) => (shutMinutes = Math.round(v))} />
            <span class="soft">minutes</span>
            <span class="spacer"></span>
            <button class="btn primary" onclick={() => startShutdown(shutMinutes, shutRestart)}>Start</button>
          </div>
          <div class="presets">
            {#each SHUT_PRESETS as m}
              <button class="preset" class:on={shutMinutes === m} onclick={() => (shutMinutes = m)}>
                {m < 60 ? `${m} min` : `${m / 60} h`}
              </button>
            {/each}
          </div>
        {/if}
        <p class="warn-note">Other apps are closed when it goes off, so save anything you're working on. The panic key cancels it.</p>
      </section>

      <section class="group glass">
        <h3>Startup &amp; tray</h3>
        <div class="row">
          <div class="text"><b>Start with Windows</b><small>Opens quietly in the tray when you sign in</small></div>
          <Toggle checked={startup} onchange={setStartup} label="Start with Windows" />
        </div>
        <div class="row">
          <div class="text"><b>Run a macro on start</b><small>Plays 5 seconds after hesytools opens</small></div>
          <select value={settings.startupMacro ?? ""} onchange={(e) => setStartupMacro(e.currentTarget.value)}>
            <option value="">None</option>
            {#each activity.macros as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
          </select>
        </div>
        <div class="row">
          <div class="text"><b>Keep running in the tray</b><small>Closing the window keeps hotkeys and watches working</small></div>
          <Toggle
            checked={settings.closeToTray}
            onchange={(v) =>
              run(async () => {
                await api.setCloseToTray(v);
                settings.closeToTray = v;
              })}
            label="Keep running in the tray"
          />
        </div>
        <div class="row">
          <div class="text"><b>Quit hesytools</b><small>Fully exits: hotkeys and watches stop</small></div>
          <button class="btn" onclick={() => api.quitApp()}>Quit</button>
        </div>
      </section>

      <section class="group glass">
        <h3>Updates</h3>
        <div class="row">
          <div class="text">
            <b>hesytools {info?.version ?? ""}</b>
            <small>
              {#if update.info && update.phase !== "idle"}Version {update.info.version} is available{:else}New versions come from GitHub Releases{/if}
            </small>
          </div>
          {#if update.info && update.phase !== "idle"}
            <button class="btn primary" onclick={() => (update.dismissed = false)}>Show update</button>
          {:else}
            <button class="btn" disabled={update.checking} onclick={() => checkForUpdates(true)}>
              {update.checking ? "Checking…" : "Check for updates"}
            </button>
          {/if}
        </div>
        <div class="row">
          <div class="text"><b>Check automatically</b><small>Looks for a new version each time hesytools starts</small></div>
          <Toggle
            checked={settings.autoCheckUpdates}
            onchange={(v) =>
              run(async () => {
                await api.setAutoCheckUpdates(v);
                settings.autoCheckUpdates = v;
              })}
            label="Check for updates automatically"
          />
        </div>
      </section>

      <section class="group glass">
        <h3>Discord alerts</h3>
        <p class="sub">Get a message when a clicker or macro finishes, or a screen watch triggers.</p>
        <div class="fields">
          <input type="url" placeholder="Webhook URL: https://discord.com/api/webhooks/…" bind:value={webhook.url} spellcheck="false" />
          <input type="text" placeholder="Your Discord user ID to get pinged (optional)" bind:value={webhook.mentionUserId} />
          <div class="row tight">
            <div class="text"><small class="plain">When the autoclicker finishes a count or time limit</small></div>
            <Toggle checked={webhook.notifyClickerFinished} onchange={(v) => (webhook.notifyClickerFinished = v)} label="Clicker finished" />
          </div>
          <div class="row tight">
            <div class="text"><small class="plain">When a macro finishes all its runs</small></div>
            <Toggle checked={webhook.notifyMacroFinished} onchange={(v) => (webhook.notifyMacroFinished = v)} label="Macro finished" />
          </div>
          <div class="actions">
            <span class="how">Channel settings → Integrations → Webhooks → Copy URL</span>
            <button class="btn" disabled={!webhook.url.trim() || testing} onclick={testWebhook}>{testing ? "Sending…" : "Send test"}</button>
            <button class="btn primary" disabled={!dirty} onclick={saveWebhook}>Save</button>
          </div>
        </div>
      </section>

      <section class="group glass">
        <h3>Admin mode {#if info?.elevated}<span class="on">On</span>{/if}</h3>
        <div class="row">
          <div class="text">
            <small class="plain">
              Windows doesn't let normal apps send input to apps running as administrator. Turn this on only if something
              ignores hesytools and runs as admin.
            </small>
          </div>
          {#if !info?.elevated}
            <button class="btn" onclick={() => run(() => api.relaunchAsAdmin())}>Restart as admin</button>
          {/if}
        </div>
      </section>

      <section class="group glass">
        <h3>Dev access {#if dev.unlocked}<span class="on">On</span>{/if}</h3>
        {#if dev.owner}
          <div class="row">
            <div class="text"><b>This is your PC</b><small>The Dev tab is always on here. Make codes for friends in the Dev tab.</small></div>
          </div>
        {:else if dev.unlocked}
          <div class="row">
            <div class="text"><b>Unlocked for {dev.name}</b><small>The Dev tab is next to Macro and Screen Watch.</small></div>
            <button class="btn" onclick={lockDev}>Remove code</button>
          </div>
        {:else}
          <p class="sub">Have an access code? Paste it here to turn on the Dev tab.</p>
          <div class="code-line">
            <input type="text" placeholder="HT-…" bind:value={devCode} spellcheck="false" autocomplete="off" />
            <button class="btn primary" disabled={!devCode.trim()} onclick={unlockDev}>Unlock</button>
          </div>
        {/if}
      </section>

      <section class="group glass about">
        <h3>About &amp; disclaimer</h3>
        <p>
          hesytools sends simulated keyboard and mouse input and reads your screen locally on your PC. Nothing leaves your
          computer except Discord alerts you set up yourself.
        </p>
        <p>
          Some games and apps don't allow automation in their terms of service, and games with anti-cheat may detect it. You're
          responsible for where you use it. Never use it in games with kernel-level anti-cheat.
        </p>
        <p class="ver">hesytools {info?.version ?? ""}</p>
      </section>
    </div>
  </div>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head {
    padding: 0 4px;
  }
  h2 {
    font-size: 22px;
    letter-spacing: -0.025em;
  }
  .head p {
    color: var(--text-3);
    font-size: 13px;
    margin-top: 3px;
  }
  .code-line {
    display: flex;
    gap: 8px;
    padding-top: 4px;
  }
  .code-line input {
    flex: 1;
    min-width: 0;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 12.5px;
  }
  .cols {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
    gap: 16px;
    align-items: start;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .group {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  h3 {
    font-size: 15px;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }
  .sub {
    font-size: 12.5px;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .label {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
    margin: 12px 0 2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--stroke);
  }
  .row.tight {
    border: none;
    padding: 4px 0;
  }
  h3 + .row,
  .sub + .row,
  .label + .row {
    border-top: none;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .text b {
    display: block;
    font-weight: 600;
    font-size: 13.5px;
  }
  .text b.red {
    color: #ff9aa8;
  }
  .text small {
    display: block;
    color: var(--text-3);
    font-size: 12px;
    margin-top: 2px;
  }
  .text small.plain {
    color: var(--text-2);
    font-size: 12.5px;
  }
  select {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid var(--stroke);
    border-radius: 10px;
    padding: 7px 10px;
    min-width: 160px;
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 6px;
  }
  .fields input {
    width: 100%;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }
  .how {
    flex: 1;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .on {
    font-size: 11px;
    color: var(--mint);
    background: rgba(61, 242, 176, 0.12);
    padding: 1px 8px;
    border-radius: 99px;
  }
  .group.armed {
    border-color: rgba(255, 92, 112, 0.45);
    box-shadow: 0 0 40px -18px rgba(255, 92, 112, 0.7);
  }
  .armed-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 0 4px;
  }
  .countdown {
    font-size: 28px !important;
    font-weight: 700 !important;
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
    color: #ff9aa8;
  }
  .shut-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 6px 0 10px;
    font-size: 14px;
  }
  .soft {
    color: var(--text-3);
  }
  .spacer {
    flex: 1;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .preset {
    font-size: 12px;
    font-weight: 550;
    padding: 4px 11px;
    border-radius: 999px;
    background: var(--glass);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .preset:hover {
    color: var(--text);
  }
  .preset.on {
    color: var(--text);
    border-color: rgba(139, 108, 255, 0.6);
    background: rgba(139, 108, 255, 0.18);
  }
  .warn-note {
    font-size: 11.5px;
    color: var(--text-3);
    margin-top: 10px;
  }
  .about p {
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.55;
    margin-top: 6px;
  }
  .about .ver {
    color: var(--text-3);
  }
</style>
