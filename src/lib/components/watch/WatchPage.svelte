<script lang="ts">
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
  import WatchPreview from "../WatchPreview.svelte";

  let { settings = $bindable() }: { settings: Settings } = $props();

  // svelte-ignore state_referenced_locally
  let rules = $state<WatchRule[]>($state.snapshot(settings.watches));
  // svelte-ignore state_referenced_locally
  let selectedId = $state<string | null>(settings.watches[0]?.id ?? null);
  let selected = $derived(rules.find((r) => r.id === selectedId));
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
      // New rules get their ids from the backend.
      const firstNew = rules.findIndex((r) => !r.id);
      rules = saved;
      if (firstNew >= 0) selectedId = saved[firstNew]?.id ?? selectedId;
      lastSaved = JSON.stringify(saved);
      settings.watches = saved;
    } catch (e) {
      toast(errorText(e), "error");
    }
  }

  let unlisten: (() => void)[] = [];
  onMount(async () => {
    unlisten = await Promise.all([
      onWatchStatus((s) => (status[s.id] = s)),
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
      condition: { kind: "textAppears", text: "" },
      pollMs: 250,
      action: { kind: "notify" },
      discord: false,
      cooldownMs: 5000,
    });
    clearTimeout(timer);
    await save();
  }

  function remove(id: string) {
    rules = rules.filter((r) => r.id !== id);
    selectedId = rules[0]?.id ?? null;
  }

  function summary(r: WatchRule): string {
    const c = r.condition;
    const when =
      c.kind === "textAppears"
        ? `“${c.text || "…"}” appears`
        : c.kind === "textGone"
          ? `“${c.text || "…"}” disappears`
          : c.kind === "number"
            ? `number ${c.op} ${c.value}`
            : c.kind === "colorAt"
              ? "a colour shows"
              : "anything changes";
    const then =
      r.action.kind === "runMacro"
        ? `run ${activity.macros.find((m) => m.id === (r.action as { macroId: string }).macroId)?.name ?? "a macro"}`
        : r.action.kind === "stopEverything"
          ? "stop everything"
          : "notify";
    return `When ${when} → ${then}`;
  }

  let actionLabel = $derived.by(() => {
    const a = selected?.action;
    if (!a) return "";
    if (a.kind === "runMacro") return `run “${activity.macros.find((m) => m.id === a.macroId)?.name ?? "pick a macro"}”`;
    return a.kind === "stopEverything" ? "stop everything" : "just notify me";
  });
</script>

{#if rules.length === 0}
  <WatchPreview />
  <div class="cta">
    <button class="btn primary" onclick={add}>Create your first watch</button>
  </div>
{:else}
  <div class="layout">
    <aside>
      <div class="side-head">
        <h2>Screen Watch</h2>
        <button class="btn small" onclick={add}>+ New</button>
      </div>
      {#each rules as r (r.id || r)}
        {@const st = status[r.id]}
        <button
          class="rule glass"
          class:on={r.id === selectedId}
          class:live={r.enabled}
          class:hit={flash[r.id]}
          onclick={() => (selectedId = r.id)}
        >
          <span class="row1">
            <span class="dot" class:watching={r.enabled} class:matched={r.enabled && st?.matched} class:err={!!st?.error}></span>
            <b>{r.name}</b>
          </span>
          <small>{summary(r)}</small>
          {#if r.enabled && st?.error}<small class="err-text">{st.error}</small>{/if}
        </button>
      {/each}
    </aside>

    {#if selected}
      {@const r = selected}
      <section class="detail glass">
        <div class="top">
          <input class="name" bind:value={r.name} spellcheck="false" aria-label="Watch name" />
          <label class="enable" class:on={r.enabled}>
            <span>{r.enabled ? "Watching" : "Off"}</span>
            <Toggle checked={r.enabled} onchange={(v) => (r.enabled = v)} label="Enable this watch" />
          </label>
        </div>

        <p class="sentence">
          When <ConditionEditor bind:condition={r.condition} /> in
          <span class="hug"><RegionChip bind:region={r.region} />,</span>
          <Popover width={280}>
            {#snippet trigger({ toggle, open })}
              <button class="chip" class:open onclick={toggle}>{actionLabel}</button>
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
                  onclick={() => ((r.action = { kind: "runMacro", macroId: m.id }), close())}
                >
                  <b>{m.name}</b>
                </button>
              {:else}
                <p class="none">No macros yet. Record one on the Macros tab.</p>
              {/each}
            {/snippet}
          </Popover>
        </p>

        <div class="opts">
          <label class="opt"><Toggle checked={r.discord} onchange={(v) => (r.discord = v)} label="Discord alert" /> Also send a Discord alert</label>
          <span class="opt">Check every <InlineNumber value={r.pollMs} min={50} onchange={(v) => (r.pollMs = Math.round(v))} /> ms</span>
          <span class="opt">
            Wait <InlineNumber value={r.cooldownMs / 1000} min={0} onchange={(v) => (r.cooldownMs = Math.round(v * 1000))} /> s before it can trigger again
          </span>
        </div>

        <div class="viewer-head">
          <span class="eyebrow">Live view</span>
          <span class="hint">Triggers when this changes from “doesn't match” to “matches”.</span>
        </div>
        <WatchViewer
          region={r.region}
          condition={r.condition}
          onpickcolor={r.condition.kind === "colorAt"
            ? (dx, dy, rgb) => {
                if (r.condition.kind === "colorAt") Object.assign(r.condition, { dx, dy, rgb });
              }
            : undefined}
        />

        <div class="foot">
          <button class="del" onclick={() => remove(r.id)}>Delete watch</button>
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .cta {
    display: flex;
    justify-content: center;
  }
  .layout {
    display: grid;
    grid-template-columns: 270px 1fr;
    gap: 18px;
    align-items: start;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .side-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 4px 6px;
  }
  h2 {
    font-size: 19px;
    letter-spacing: -0.02em;
  }
  .btn.small {
    height: 30px;
    padding: 0 12px;
    font-size: 13px;
  }
  .rule {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    padding: 12px 14px;
    border-radius: 16px;
    transition:
      border-color 250ms var(--ease-smooth-out),
      box-shadow 400ms var(--ease-smooth-out);
  }
  .rule.on {
    border-color: rgba(56, 214, 255, 0.5);
  }
  .rule.hit {
    box-shadow: 0 0 30px -6px rgba(61, 242, 176, 0.8);
  }
  .row1 {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row1 b {
    font-weight: 650;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rule small {
    color: var(--text-3);
    font-size: 12px;
    line-height: 1.4;
  }
  .rule .err-text {
    color: #ffb3bd;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
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
    box-shadow: 0 0 8px var(--red);
  }
  .detail {
    padding: 22px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .name {
    flex: 1;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.025em;
    padding: 2px 8px;
    margin-left: -9px;
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
    gap: 10px;
    font-weight: 600;
    font-size: 13px;
    color: var(--text-3);
    cursor: pointer;
  }
  .enable.on {
    color: var(--cyan);
  }
  .sentence {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    font-size: 16px;
    font-weight: 550;
    color: var(--text-2);
    line-height: 1.8;
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
  .hug {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .sentence :global(.pop) {
    font-size: 14px;
    line-height: 1.4;
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
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 22px;
    font-size: 13px;
    color: var(--text-2);
  }
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .viewer-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-top: 4px;
  }
  .hint {
    font-size: 12px;
    color: var(--text-3);
  }
  .foot {
    display: flex;
    justify-content: flex-end;
  }
  .del {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-3);
    padding: 6px 12px;
    border-radius: 99px;
  }
  .del:hover {
    color: var(--red);
    background: rgba(255, 92, 112, 0.08);
  }
</style>
