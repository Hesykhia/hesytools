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
    leech: true,
    blessings: false,
    frenzy: true,
    buyGens: false,
    buyUpgrades: false,
    orb: false,
    orbCps: 8,
    reserveMinutes: 0,
    treeStop: true,
    treeBuy: false,
    secrets: true,
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
  let bundled = $state("");
  onMount(() => api.soulHelperVersion().then((v) => (bundled = v)));
  let outdated = $derived(connected && !!bundled && (report?.data.version ?? "1.0.0") !== bundled);
  let gain = $derived(ritual && ritual.can ? (100 + ritual.bonus + ritual.pendingBonus) / (100 + ritual.bonus) : null);

  const install = async () => {
    try {
      await api.installSoulHelper();
    } catch (e) {
      toast(errorText(e), "error");
    }
  };
  // ── Owner: codes for friends ──
  let friend = $state("");
  let fresh = $state<string | null>(null);
  async function makeCode() {
    try {
      settings = await api.makeDevCode(friend);
      fresh = friend.trim();
      friend = "";
    } catch (e) {
      toast(errorText(e), "error");
    }
  }
  async function copy(code: string) {
    try {
      await navigator.clipboard.writeText(code);
      toast("Code copied. Send it to them; they paste it in Settings → Dev access.");
    } catch {
      toast("Couldn't copy. Select the code and copy it by hand.", "error");
    }
  }
  const forget = (name: string) =>
    api
      .forgetDevCode(name)
      .then((s) => (settings = s))
      .catch((e) => toast(errorText(e), "error"));
  const when = (ms: number) => new Date(ms).toLocaleDateString([], { month: "short", day: "numeric" });

  const open = (url: string) => api.openLink(url).catch((e) => toast(errorText(e), "error"));

  const AUTOMATIONS: {
    key: "favors" | "crystals" | "leech" | "blessings" | "buyUpgrades" | "buyGens" | "secrets";
    name: string;
    what: string;
  }[] = [
    { key: "favors", name: "Catch favors", what: "Every Golden Statue and buff favor, plus the soul spheres chains and storms scatter" },
    { key: "crystals", name: "Kill Mid-Bosses", what: "For their Rejuv Crystals" },
    { key: "leech", name: "Claim the Soul Leech", what: "What your sources earned while you were away" },
    { key: "blessings", name: "Release banked blessings", what: "Use a saved blessing as soon as it's banked" },
    {
      key: "buyUpgrades",
      name: "Buy Curiosity Shop items",
      what: "The ones that help this run most, ranked with the generators. Skips click items when nothing is clicking or on an Empty Hand run",
    },
    {
      key: "buyGens",
      name: "Buy generators",
      what: "Whatever pays off soonest against everything you make, including saving up for the next 10/25/50… milestone that doubles a generator",
    },
    {
      key: "secrets",
      name: "Collect free secrets",
      what: "Each secret is +1% output. Tries on every look you've unlocked once and puts yours back, opens the Options once, holds exactly 100 Patrons for a moment, and so on",
    },
  ];
  const RITUAL: { value: RitualMode; label: string }[] = [
    { value: "off", label: "Off" },
    { value: "fastest", label: "Fastest growth" },
    { value: "worth", label: "Worth it" },
    { value: "shards", label: "At least" },
    { value: "always", label: "Every time" },
  ];
</script>

<div class="page">
  {#snippet giveAccess()}
    <section class="access glass">
      <div class="top">
        <div class="title"><h3>Give access</h3></div>
      </div>
      <p class="explain soft">Type a friend's name and make them a code. Only your PC can make codes, and each one says who it's for.</p>
      <div class="make">
        <input type="text" placeholder="Their name" bind:value={friend} maxlength="40" onkeydown={(e) => e.key === "Enter" && friend.trim() && makeCode()} />
        <button class="btn primary" disabled={!friend.trim()} onclick={makeCode}>Make code</button>
      </div>
      {#if settings.issuedCodes?.length}
        <ul class="codes">
          {#each settings.issuedCodes as c (c.name)}
            <li class:fresh={fresh === c.name}>
              <b>{c.name}</b>
              <code>{c.code}</code>
              <span class="when">{when(c.madeAt)}</span>
              <button class="btn" onclick={() => copy(c.code)}>Copy</button>
              <button class="x" title="Remove from this list (the code still works)" aria-label="Remove" onclick={() => forget(c.name)}>✕</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/snippet}
  <header class="head">
    <div>
      <h2>Dev</h2>
      <p>{dev.owner ? "Your PC: always unlocked. Friends need a code from you." : `Unlocked for ${dev.name}. Only people with a code can see this.`}</p>
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

    {#if outdated}
      <div class="update">
        <span>The helper in your browser is older than this version of hesytools.</span>
        <button class="btn primary" onclick={install}>Update helper</button>
      </div>
    {/if}

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
      {#if stats?.plan && cfg.buyGens}
        <p class="plan">Plan: {stats.plan}</p>
      {/if}
      {#if stats}
        <p class="tally">
          {stats.favors} favors · {stats.spheres ?? 0} spheres · {stats.crystals} Mid-Bosses · {stats.upgrades} upgrades ·
          {stats.treeNodes ?? 0} tree nodes · {stats.secrets ?? 0} secrets · {stats.gens} generators · {stats.rituals}
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
        <div class="row">
          <div class="text">
            <b>Go all out during click blessings</b>
            <small>While something like Trigger Frenzy is running, click the orb 20 times a second (even if the above is off)</small>
          </div>
          <Toggle checked={cfg.frenzy} onchange={(v) => (cfg.frenzy = v)} label="Click hard during click blessings" />
        </div>
        <div class="row">
          <div class="text">
            <b>Keep a reserve</b>
            <small>
              Don't spend the last <InlineNumber value={cfg.reserveMinutes} min={0} onchange={(v) => (cfg.reserveMinutes = Math.max(0, v))} />
              minutes of income. Lucky Souls pays up to 15% of what you're holding, so a reserve makes it pay more. 0 = spend it all.
            </small>
          </div>
        </div>
      </div>

      <div class="col">
        <span class="label">The Ritual</span>
        <Tabs size="s" value={cfg.ritual.mode} onchange={(v) => (cfg.ritual.mode = v as RitualMode)} options={RITUAL} />
        <p class="explain">
          {#if cfg.ritual.mode === "off"}
            You do the Ritual yourself.
          {:else if cfg.ritual.mode === "fastest"}
            Watches how fast this run is growing your permanent bonus, per minute of the run, and sacrifices just after that
            peaks. Over many runs that grows your bonus fastest. Recommended.
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
        <div class="row tree">
          <div class="text">
            <b>Spend shards on the tree</b>
            <small>
              Buys the nodes worth the most per shard (more output, better shard rate, more shards per Ritual, boosts to what
              your runs are made of). Saves up when something much better is close. Counts cosmetic nodes by the secrets they open
              when Collect free secrets is on.
            </small>
          </div>
          <Toggle checked={cfg.treeBuy} onchange={(v) => (cfg.treeBuy = v)} label="Spend shards on the tree" />
        </div>
        {#if cfg.ritual.mode !== "off"}
          <div class="row tree">
            <div class="text">
              <b>Stop on the skill tree</b>
              <small>
                The tree only opens right after a Ritual. Leave it up so you can spend shards, then press Return to the run. Off =
                carry straight on (good when you're away).
              </small>
            </div>
            <Toggle checked={cfg.treeStop} onchange={(v) => (cfg.treeStop = v)} label="Stop on the skill tree after a Ritual" />
          </div>
        {/if}
      </div>
    </div>

    <p class="foot">
      It only reads the page and presses the game's own buttons, like you would. It never edits your souls or save and never
      uses the console, so nothing marks your save as cheated.
    </p>
  </section>

  {#if dev.owner}{@render giveAccess()}{/if}
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
  .update {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 12px;
    font-size: 13px;
    color: var(--amber);
    background: rgba(255, 191, 92, 0.08);
    border: 1px solid rgba(255, 191, 92, 0.35);
  }
  .update span {
    flex: 1;
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
  .plan {
    font-size: 13px;
    font-weight: 600;
    color: var(--violet-2);
    margin-top: -6px;
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
  .row.tree {
    margin-top: 10px;
  }
  .explain.soft {
    color: var(--text-3);
    font-size: 12px;
    margin-top: 6px;
  }
  .access {
    padding: 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .access .explain {
    margin-top: 0;
  }
  .make {
    display: flex;
    gap: 8px;
  }
  .make input {
    flex: 1;
    max-width: 320px;
  }
  .codes {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }
  .codes li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    border-radius: 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--stroke);
    transition: border-color 300ms var(--ease-smooth-out);
  }
  .codes li.fresh {
    border-color: rgba(61, 242, 176, 0.5);
  }
  .codes b {
    font-size: 13.5px;
    min-width: 90px;
  }
  .codes code {
    flex: 1;
    min-width: 0;
    font-family: ui-monospace, Consolas, monospace;
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: all;
  }
  .when {
    font-size: 12px;
    color: var(--text-3);
  }
  .x {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    color: var(--text-3);
  }
  .x:hover {
    color: var(--text);
    background: var(--glass-hover);
  }
  .foot {
    font-size: 12px;
    color: var(--text-3);
    padding-top: 12px;
    border-top: 1px solid var(--stroke);
  }
</style>
