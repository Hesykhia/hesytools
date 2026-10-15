<script lang="ts">
  // The Dev tab (needs an access code): the Soul Clicker helper. A small
  // userscript in the browser presses the game's own buttons; this page sets
  // what it does and shows what it's seeing.
  import { onDestroy, onMount } from "svelte";
  import { api, errorText, type RitualMode, type Settings, type SoulConfig } from "$lib/api";
  import { dev } from "$lib/dev.svelte";
  import { toast } from "$lib/toast.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Tabs from "../ui/Tabs.svelte";
  import Toggle from "../ui/Toggle.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

  const DEFAULTS: SoulConfig = {
    enabled: false,
    favors: true,
    crystals: true,
    buyGens: false,
    buyUpgrades: false,
    orb: false,
    orbCps: 8,
    ritual: { mode: "off", worth: 1.5, shards: 10 },
  };
  // svelte-ignore state_referenced_locally
  let cfg = $state<SoulConfig>({ ...DEFAULTS, ...$state.snapshot(settings.soul ?? {}), ritual: { ...DEFAULTS.ritual, ...(settings.soul?.ritual ?? {}) } });

  // Save shortly after any change; the helper picks it up within a second.
  let lastSaved = JSON.stringify(cfg);
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const snap = JSON.stringify(cfg);
    if (snap === lastSaved) return;
    clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        settings = await api.setSoulConfig($state.snapshot(cfg));
        lastSaved = snap;
      } catch (e) {
        toast(errorText(e), "error");
      }
    }, 250);
  });

  // "Connected" = the helper reported in the last few seconds.
  let now = $state(Date.now());
  let clock: ReturnType<typeof setInterval>;
  onMount(() => (clock = setInterval(() => (now = Date.now()), 1000)));
  onDestroy(() => clearInterval(clock));
  let report = $derived(dev.report);
  let connected = $derived(!!report && now - report.at < 4000);
  let status = $derived(report?.data.status);
  let stats = $derived(report?.data.stats);
  let ritual = $derived(report?.data.ritual);
  let gain = $derived(ritual && ritual.can ? (100 + ritual.bonus + ritual.pendingBonus) / (100 + ritual.bonus) : null);

  const install = async () => {
    try {
      await api.installSoulHelper();
    } catch (e) {
      toast(errorText(e), "error");
    }
  };
  const open = (url: string) => api.openLink(url).catch((e) => toast(errorText(e), "error"));

  const AUTOMATIONS: { key: "favors" | "crystals" | "buyUpgrades" | "buyGens"; name: string; what: string }[] = [
    { key: "favors", name: "Catch favors", what: "Golden Statues the moment they appear" },
    { key: "crystals", name: "Kill Mid-Bosses", what: "For their Rejuv Crystals" },
    { key: "buyUpgrades", name: "Buy upgrades", what: "Anything in the Curiosity Shop you can afford" },
    { key: "buyGens", name: "Buy generators", what: "Always the best value: most souls per second for the price" },
  ];
  const RITUAL: { value: RitualMode; label: string }[] = [
    { value: "off", label: "Off" },
    { value: "worth", label: "When it's worth it" },
    { value: "shards", label: "At least" },
    { value: "always", label: "Every time" },
  ];
</script>

<div class="page">
  <header class="head">
    <div>
      <h2>Dev</h2>
      <p>Unlocked for {dev.name}. Only people with a code can see this.</p>
    </div>
  </header>

  <section class="game glass" class:live={connected && cfg.enabled}>
    <div class="top">
      <div class="title">
        <h3>Soul Clicker</h3>
        <span class="pill" class:ok={connected}>
          <span class="dot"></span>{connected ? "Helper connected" : "Helper not connected"}
        </span>
      </div>
      <label class="master" class:on={cfg.enabled}>
        {cfg.enabled ? "Running" : "Paused"}
        <Toggle checked={cfg.enabled} onchange={(v) => (cfg.enabled = v)} label="Run the Soul Clicker helper" />
      </label>
    </div>

    {#if !connected}
      <ol class="setup">
        <li>
          <b>1</b><span>Install <strong>Violentmonkey</strong> in Firefox (free, runs small scripts on pages you choose).</span>
          <button class="btn" onclick={() => open("https://addons.mozilla.org/firefox/addon/violentmonkey/")}>Get Violentmonkey</button>
        </li>
        <li>
          <b>2</b><span>Install the helper. Violentmonkey opens a page; press <strong>Install</strong>.</span>
          <button class="btn primary" onclick={install}>Install helper</button>
        </li>
        <li>
          <b>3</b><span>Open Soul Clicker in Firefox. This turns green once the helper says hi.</span>
          <button class="btn" onclick={() => open("https://www.deadchaps.gg/soul-clicker")}>Open the game</button>
        </li>
      </ol>
    {:else if status}
      <div class="stats">
        <div class="stat big"><small>Souls</small><b>{status.souls || "0"}</b></div>
        <div class="stat"><small>Per second</small><b>{status.sps || "0"}</b></div>
        <div class="stat"><small>Per click</small><b>{status.perClick || "1"}</b></div>
        <div class="stat">
          <small>Ritual ready</small>
          <b>{ritual?.can ? `${ritual.pending} shards` : status.pending || "not yet"}</b>
          {#if gain}<em>×{gain.toFixed(2)} output</em>{/if}
        </div>
      </div>
      {#if stats}
        <p class="tally">
          {stats.favors} favors · {stats.crystals} Mid-Bosses · {stats.upgrades} upgrades · {stats.gens} generators · {stats.rituals}
          Rituals this session{#if stats.last}<span class="last">&nbsp;· {stats.last}</span>{/if}
        </p>
      {/if}
    {/if}

    <div class="cols">
      <div class="col">
        <span class="label">Automate</span>
        {#each AUTOMATIONS as a}
          <div class="row">
            <div class="text"><b>{a.name}</b><small>{a.what}</small></div>
            <Toggle checked={cfg[a.key]} onchange={(v) => (cfg[a.key] = v)} label={a.name} />
          </div>
        {/each}
        <div class="row">
          <div class="text">
            <b>Click the orb</b>
            <small>
              <InlineNumber value={cfg.orbCps} min={1} max={20} onchange={(v) => (cfg.orbCps = Math.round(v))} /> times a second, with an
              uneven rhythm so the game's autoclicker check doesn't trip
            </small>
          </div>
          <Toggle checked={cfg.orb} onchange={(v) => (cfg.orb = v)} label="Click the orb" />
        </div>
      </div>

      <div class="col">
        <span class="label">The Ritual</span>
        <Tabs size="s" value={cfg.ritual.mode} onchange={(v) => (cfg.ritual.mode = v as RitualMode)} options={RITUAL} />
        <p class="explain">
          {#if cfg.ritual.mode === "off"}
            You do the Ritual yourself.
          {:else if cfg.ritual.mode === "worth"}
            Sacrifice once your output afterwards would be at least
            <InlineNumber value={cfg.ritual.worth} min={1.01} onchange={(v) => (cfg.ritual.worth = Math.round(v * 100) / 100)} />× what it
            is now. Each shard earned adds to your permanent bonus (2% each, more with some tree nodes).
          {:else if cfg.ritual.mode === "shards"}
            Sacrifice once at least <InlineNumber value={cfg.ritual.shards} min={1} onchange={(v) => (cfg.ritual.shards = Math.round(v))} />
            shards are waiting.
          {:else}
            Sacrifice as soon as a single shard is waiting, every time.
          {/if}
        </p>
        <p class="explain soft">Spending shards on the tree stays up to you.</p>
      </div>
    </div>

    <p class="foot">
      It only reads the page and presses the game's own buttons, like you would. It never edits your souls or save and never
      uses the console, so nothing marks your save as cheated.
    </p>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head h2 {
    font-size: 24px;
    letter-spacing: -0.02em;
  }
  .head p {
    color: var(--text-3);
    font-size: 13px;
    margin-top: 2px;
  }
  .game {
    padding: 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    transition:
      border-color 300ms var(--ease-smooth-out),
      box-shadow 300ms var(--ease-smooth-out);
  }
  .game.live {
    border-color: rgba(61, 242, 176, 0.4);
    box-shadow: 0 0 44px -22px rgba(61, 242, 176, 0.7);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .title {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .title h3 {
    font-size: 18px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 99px;
    color: var(--text-3);
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--stroke);
  }
  .pill .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .pill.ok {
    color: var(--mint);
    box-shadow: inset 0 0 0 1px rgba(61, 242, 176, 0.4);
  }
  .pill.ok .dot {
    background: var(--mint);
    box-shadow: 0 0 8px var(--mint);
  }
  .master {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    font-weight: 650;
    color: var(--text-3);
  }
  .master.on {
    color: var(--mint);
  }
  .setup {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .setup li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--stroke);
    font-size: 13px;
    color: var(--text-2);
  }
  .setup li span {
    flex: 1;
  }
  .setup b {
    width: 22px;
    height: 22px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-size: 11.5px;
    color: var(--cyan);
    box-shadow: inset 0 0 0 1px rgba(56, 214, 255, 0.45);
  }
  .setup strong {
    color: var(--text);
  }
  .stats {
    display: grid;
    grid-template-columns: 1.6fr 1fr 1fr 1.2fr;
    gap: 10px;
  }
  .stat {
    padding: 12px 14px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.035);
    border: 1px solid var(--stroke);
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .stat small {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .stat b {
    font-size: 17px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .stat.big b {
    font-size: 22px;
    color: var(--mint);
  }
  .stat em {
    font-style: normal;
    font-size: 12px;
    color: var(--violet-2);
    font-weight: 600;
  }
  .tally {
    font-size: 12.5px;
    color: var(--text-3);
    margin-top: -6px;
  }
  .tally .last {
    color: var(--text-2);
  }
  .cols {
    display: grid;
    grid-template-columns: 1.3fr 1fr;
    gap: 28px;
  }
  .col {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 0;
    border-top: 1px solid var(--stroke);
  }
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
  .text small {
    display: block;
    color: var(--text-3);
    font-size: 12px;
    margin-top: 2px;
  }
  .explain {
    font-size: 13px;
    color: var(--text-2);
    margin-top: 12px;
    line-height: 1.55;
  }
  .explain.soft {
    color: var(--text-3);
    font-size: 12px;
    margin-top: 6px;
  }
  .foot {
    font-size: 12px;
    color: var(--text-3);
    padding-top: 12px;
    border-top: 1px solid var(--stroke);
  }
</style>
