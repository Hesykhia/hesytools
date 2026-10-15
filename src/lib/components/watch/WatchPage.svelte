<script lang="ts">
  // Screen Watch on one page: every watch is shown in full (where / what /
  // then, with its live view beside it). Nothing to click into.
  import { onDestroy, onMount } from "svelte";
  import { api, errorText, onWatchHit, onWatchStatus, type Settings, type WatchRule, type WatchStatus } from "$lib/api";
  import { activity } from "$lib/activity.svelte";
  import { toast } from "$lib/toast.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";
  import Toggle from "../ui/Toggle.svelte";
  import ConditionEditor from "./ConditionEditor.svelte";
  import RegionChip from "./RegionChip.svelte";
  import WatchViewer from "./WatchViewer.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

  // svelte-ignore state_referenced_locally
  let rules = $state<WatchRule[]>($state.snapshot(settings.watches));
  let status = $state<Record<string, WatchStatus>>({});
  /** Watches that just triggered (glow for a moment). */
  let flash = $state<Record<string, boolean>>({});

  // ── Persist (debounced). The backend restarts watch threads on save. ──
  // svelte-ignore state_referenced_locally
  let lastSaved = JSON.stringify(rules);
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const snap = JSON.stringify(rules);
    if (snap === lastSaved) return;
    clearTimeout(timer);
    timer = setTimeout(save, 400);
  });
  async function save() {
    try {
      const saved = await api.saveWatches($state.snapshot(rules));
      rules = saved;
      lastSaved = JSON.stringify(saved);
      settings.watches = saved;
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  let unlisten: (() => void)[] = [];
  onMount(async () => {
    unlisten = await Promise.all([
      onWatchStatus((s) => {
        status[s.id] = s;
      }),
      onWatchHit((id) => {
        flash[id] = true;
        setTimeout(() => delete flash[id], 1500);
      }),
    ]);
  });
  onDestroy(() => {
    unlisten.forEach((u) => u());
    clearTimeout(timer);
    if (JSON.stringify(rules) !== lastSaved) save();
  });

  async function add() {
    rules.unshift({
      id: "",
      name: `Watch ${rules.length + 1}`,
      enabled: false,
      region: { x: 0, y: 0, w: 0, h: 0 },
      condition: { kind: "textAppears", text: "", digitsAfter: null },
      pollMs: 250,
      action: { kind: "notify" },
      discord: false,
      cooldownMs: 5000,
    });
    clearTimeout(timer);
    await save();
  }

  function actionLabel(r: WatchRule) {
    const a = r.action;
    if (a.kind === "runMacro") return `run “${activity.macros.find((m) => m.id === a.macroId)?.name ?? "pick a macro"}”`;
    return a.kind === "stopEverything" ? "stop everything" : "just notify me";
  }

  let ready = (r: WatchRule) => r.region.w > 0 && r.region.h > 0;
</script>

<div class="page">
  <header class="head">
    <div>
      <h2>Screen Watch</h2>
      <p>Watch part of your screen and react the moment something shows up.</p>
    </div>
    <button class="btn primary" onclick={add}>+ Add a watch</button>
  </header>

  <ol class="how">
    <li><b>1</b><span><strong>Select an area</strong> of the screen, e.g. where your coins or a button appear</span></li>
    <li><b>2</b><span><strong>Choose what to look for</strong>: text, a number, a colour or an image</span></li>
    <li><b>3</b><span><strong>Choose what happens</strong>: run a macro, stop everything, or get notified</span></li>
  </ol>

  {#each rules as r (r.id || r)}
    {@const st = status[r.id]}
    <article class="watch glass" class:on={r.enabled} class:hit={flash[r.id]}>
      <div class="form">
        <div class="top">
          <span class="dot" class:watching={r.enabled} class:matched={r.enabled && st?.matched} class:err={!!st?.error}></span>
          <input class="name" type="text" bind:value={r.name} spellcheck="false" aria-label="Watch name" />
          <label class="enable" class:on={r.enabled}>
            {r.enabled ? "Watching" : "Off"}
            <Toggle checked={r.enabled} onchange={(v) => (r.enabled = v)} label="Turn this watch on" />
          </label>
        </div>

        <div class="step">
          <span class="n">1</span>
          <div>
            <span class="label">Where</span>
            <div class="line">
              <RegionChip bind:region={r.region} />
              {#if ready(r)}<span class="soft">at {r.region.x}, {r.region.y}</span>{:else}<span class="soft">drag a box on any monitor</span>{/if}
            </div>
          </div>
        </div>

        <div class="step">
          <span class="n">2</span>
          <div>
            <span class="label">Look for</span>
            <div class="line"><ConditionEditor bind:condition={r.condition} /></div>
          </div>
        </div>

        <div class="step">
          <span class="n">3</span>
          <div>
            <span class="label">Then</span>
            <div class="line">
              <Popover width={280}>
                {#snippet trigger({ toggle, open })}
                  <button class="chip" class:open onclick={toggle}>{actionLabel(r)}</button>
                {/snippet}
                {#snippet children({ close })}
                  <button class="item" class:on={r.action.kind === "notify"} onclick={() => ((r.action = { kind: "notify" }), close())}>
                    <b>Just notify me</b><small>A pop-up here (and Discord if on)</small>
                  </button>
                  <button class="item" class:on={r.action.kind === "stopEverything"} onclick={() => ((r.action = { kind: "stopEverything" }), close())}>
                    <b>Stop everything</b><small>Like pressing the panic key</small>
                  </button>
                  <div class="eyebrow pad">Run a macro</div>
                  {#each activity.macros as m}
                    <button
                      class="item"
                      class:on={r.action.kind === "runMacro" && r.action.macroId === m.id}
                      onclick={() => ((r.action = { kind: "runMacro", macroId: m.id }), close())}><b>{m.name}</b></button
                    >
                  {:else}
                    <p class="none">No macros yet. Record one on the Macros tab.</p>
                  {/each}
                {/snippet}
              </Popover>
              <label class="opt"><Toggle checked={r.discord} onchange={(v) => (r.discord = v)} label="Discord alert" /> + Discord alert</label>
            </div>
          </div>
        </div>

        <div class="foot">
          <span class="opt">Check every <InlineNumber value={r.pollMs} min={50} onchange={(v) => (r.pollMs = Math.round(v))} /> ms</span>
          <span class="opt">
            Wait <InlineNumber value={r.cooldownMs / 1000} min={0} onchange={(v) => (r.cooldownMs = Math.round(v * 1000))} /> s before triggering
            again
          </span>
          <span class="spacer"></span>
          <button class="del" onclick={() => (rules = rules.filter((x) => x !== r))}>Delete</button>
        </div>
        {#if r.enabled && st?.error}<p class="err">{st.error}</p>{/if}
      </div>

      <div class="viewer">
        <span class="label">Live view</span>
        <WatchViewer
          region={ready(r) ? r.region : null}
          condition={r.condition}
          onpickcolor={r.condition.kind === "colorAt" || r.condition.kind === "colorAppears"
            ? (dx, dy, rgb) => {
                if (r.condition.kind === "colorAt") Object.assign(r.condition, { dx, dy, rgb });
                if (r.condition.kind === "colorAppears") r.condition.rgb = rgb;
              }
            : undefined}
        />
        <p class="hint">It triggers when this changes from “doesn't match” to “matches”.</p>
      </div>
    </article>
  {:else}
    <div class="empty glass">
      <p>No watches yet.</p>
      <button class="btn primary" onclick={add}>+ Add your first watch</button>
    </div>
  {/each}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
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
  .how {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .how li {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 12px 14px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--stroke);
    font-size: 13px;
    color: var(--text-3);
    line-height: 1.45;
  }
  .how strong {
    color: var(--text);
    font-weight: 600;
  }
  .how b,
  .step .n {
    flex: none;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    font-size: 12px;
    font-weight: 700;
    color: var(--cyan);
    background: rgba(56, 214, 255, 0.12);
    border: 1px solid rgba(56, 214, 255, 0.35);
  }
  .watch {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(300px, 1fr);
    gap: 26px;
    padding: 20px 22px;
    transition:
      border-color 300ms var(--ease-smooth-out),
      box-shadow 400ms var(--ease-smooth-out);
  }
  .watch.on {
    border-color: rgba(56, 214, 255, 0.35);
  }
  .watch.hit {
    box-shadow: 0 0 40px -10px rgba(61, 242, 176, 0.8);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    flex: 1;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.02em;
    padding: 2px 8px;
    background: transparent !important;
    border-color: transparent !important;
    box-shadow: none !important;
  }
  .name:hover {
    background: var(--glass) !important;
  }
  .enable {
    display: flex;
    align-items: center;
    gap: 9px;
    font-weight: 600;
    font-size: 13px;
    color: var(--text-3);
    cursor: pointer;
  }
  .enable.on {
    color: var(--cyan);
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--text-3);
    opacity: 0.5;
  }
  .dot.watching {
    background: var(--cyan);
    opacity: 1;
    box-shadow: 0 0 8px var(--cyan);
  }
  .dot.matched {
    background: var(--mint);
    box-shadow: 0 0 8px var(--mint);
  }
  .dot.err {
    background: var(--red);
  }
  .step {
    display: flex;
    gap: 12px;
  }
  .step > div {
    flex: 1;
    min-width: 0;
  }
  .label {
    display: block;
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
    margin-bottom: 6px;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font-size: 14.5px;
    font-weight: 550;
  }
  .soft {
    color: var(--text-3);
    font-size: 12.5px;
    font-weight: 500;
  }
  .chip {
    font-weight: 650;
    padding: 2px 10px;
    border-radius: 8px;
    color: var(--text);
    background: rgba(139, 108, 255, 0.18);
    box-shadow: inset 0 0 0 1px rgba(139, 108, 255, 0.4);
  }
  .chip:hover,
  .chip.open {
    background: rgba(139, 108, 255, 0.3);
  }
  .item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    border-radius: 9px;
  }
  .item:hover {
    background: var(--glass-hover);
  }
  .item b {
    display: block;
    font-weight: 600;
    font-size: 13.5px;
  }
  .item small {
    color: var(--text-3);
    font-size: 12px;
  }
  .item.on b {
    color: var(--violet-2);
  }
  .pad {
    padding: 10px 10px 4px;
  }
  .none {
    font-size: 12.5px;
    color: var(--text-3);
    padding: 4px 10px 6px;
  }
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    color: var(--text-2);
    font-weight: 500;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 20px;
    padding-top: 12px;
    border-top: 1px solid var(--stroke);
  }
  .spacer {
    flex: 1;
  }
  .del {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-3);
    padding: 5px 12px;
    border-radius: 99px;
  }
  .del:hover {
    color: var(--red);
    background: rgba(255, 92, 112, 0.08);
  }
  .err {
    font-size: 12.5px;
    color: #ffb3bd;
  }
  .viewer {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .hint {
    font-size: 11.5px;
    color: var(--text-3);
    margin-top: 8px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 40px;
    color: var(--text-3);
  }
  @media (max-width: 900px) {
    .watch {
      grid-template-columns: 1fr;
    }
    .how {
      grid-template-columns: 1fr;
    }
  }
</style>
