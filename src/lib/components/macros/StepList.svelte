<script lang="ts">
  import { tick } from "svelte";
  import { api, errorText, MOUSE_BUTTONS, type MouseButton, type PressAction, type Step, type StepType } from "$lib/api";
  import { BUTTON_LABEL, defaultStep, durationMs, formatMs, STEP_TYPES } from "$lib/steps";
  import { toast } from "$lib/toast.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";
  import StepIcon from "../ui/StepIcon.svelte";
  import StepList from "./StepList.svelte";
  import ConditionEditor from "../watch/ConditionEditor.svelte";
  import RegionChip from "../watch/RegionChip.svelte";
  import WatchViewer from "../watch/WatchViewer.svelte";

  let {
    steps,
    active = -1,
    depth = 0,
  }: {
    /** Mutated in place; the editor autosaves on any change. */
    steps: Step[];
    /** Index of the step currently playing (top level only). */
    active?: number;
    depth?: number;
  } = $props();

  let rows: HTMLElement[] = $state([]);
  let actionNumber = $derived.by(() => {
    // Waits don't get a number; they read as the gaps between actions.
    let n = 0;
    return steps.map((s) => (s.type === "wait" ? 0 : ++n));
  });

  $effect(() => {
    if (active >= 0) rows[active]?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= steps.length) return;
    const [s] = steps.splice(i, 1);
    steps.splice(j, 0, s);
  }

  async function add(type: StepType, close: () => void) {
    steps.push(defaultStep(type));
    close();
    await tick();
    rows[steps.length - 1]?.querySelector<HTMLElement>("input, .chip")?.focus();
  }

  /** Screen-wait step whose live view is open. */
  let viewing = $state<Step | null>(null);
  let capturing = $state(-1);
  async function captureKey(i: number) {
    const s = steps[i];
    if (s.type !== "key") return;
    capturing = i;
    try {
      const combo = await api.captureHotkey();
      if (!combo) return;
      const trigger = combo.split("+").pop()!;
      if (MOUSE_BUTTONS.some((b) => b.value === trigger)) {
        toast("That's a mouse button. Use a Click step for it.");
        return;
      }
      s.key = trigger;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      capturing = -1;
    }
  }

  let picking = $state(-1);
  async function pickInto(i: number) {
    picking = i;
    try {
      const p = await api.pickPoint();
      if (!p) return;
      const s = steps[i];
      if (s.type === "click") s.at = p;
      if (s.type === "move") [s.x, s.y] = p;
    } finally {
      picking = -1;
    }
  }

  const ACTIONS: { value: PressAction; label: string }[] = [
    { value: "tap", label: "Press" },
    { value: "down", label: "Hold down" },
    { value: "up", label: "Release" },
  ];
  const keyVerb = (a: PressAction) => ({ tap: "Press", down: "Hold down", up: "Release" })[a];
  const clickVerb = (b: MouseButton, a: PressAction) =>
    a === "tap" ? BUTTON_LABEL[b] : `${a === "down" ? "Hold" : "Release"} ${BUTTON_LABEL[b].replace(" click", "")}`;
</script>

<ol class="list" class:nested={depth > 0}>
  {#each steps as s, i (s)}
    {#if s.type === "wait"}
      <li class="wait" bind:this={rows[i]} class:active={i === active}>
        <span class="rail"></span>
        <span class="wait-pill">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z" /></svg>
          wait <InlineNumber value={s.ms} onchange={(v) => (s.ms = v)} /> ms
        </span>
        <span class="tools">
          <button title="Move up" onclick={() => move(i, -1)} disabled={i === 0}>↑</button>
          <button title="Move down" onclick={() => move(i, 1)} disabled={i === steps.length - 1}>↓</button>
          <button title="Delete" class="del" onclick={() => steps.splice(i, 1)}>✕</button>
        </span>
      </li>
    {:else}
      <li class="row" bind:this={rows[i]} class:active={i === active}>
        <span class="n">{actionNumber[i]}</span>
        <StepIcon type={s.type} />
        <div class="say">
          {#if s.type === "key"}
            <Popover width={200}>
              {#snippet trigger({ toggle, open })}
                <button class="chip" class:open onclick={toggle}>{keyVerb(s.action)}</button>
              {/snippet}
              {#snippet children({ close })}
                {#each ACTIONS as a}
                  <button class="menu-item" class:on={s.action === a.value} onclick={() => ((s.action = a.value), close())}>{a.label}</button>
                {/each}
              {/snippet}
            </Popover>
            <button class="keycap" class:listening={capturing === i} onclick={() => captureKey(i)} title="Click, then press a key">
              {capturing === i ? "press a key…" : s.key}
            </button>
            {#if s.action === "tap"}
              <span class="soft">for</span> <InlineNumber value={s.holdMs} onchange={(v) => (s.holdMs = v)} /> <span class="soft">ms</span>
            {/if}
          {:else if s.type === "click"}
            <Popover width={300}>
              {#snippet trigger({ toggle, open })}
                <button class="chip" class:open onclick={toggle}>{clickVerb(s.button, s.action)}</button>
              {/snippet}
              {#snippet children({ close })}
                <div class="grid3">
                  {#each MOUSE_BUTTONS as b}
                    <button class="tile" class:on={s.button === b.value} onclick={() => (s.button = b.value)}>{b.label.replace(/ \(.*\)/, "")}</button>
                  {/each}
                </div>
                <div class="sep"></div>
                {#each ACTIONS as a}
                  <button class="menu-item" class:on={s.action === a.value} onclick={() => ((s.action = a.value), close())}>
                    {a.value === "tap" ? "Click" : a.label}
                  </button>
                {/each}
              {/snippet}
            </Popover>
            {#if s.at}
              <span class="soft">at</span>
              <InlineNumber value={s.at[0]} min={-1e5} onchange={(v) => s.at && (s.at[0] = Math.round(v))} />
              <InlineNumber value={s.at[1]} min={-1e5} onchange={(v) => s.at && (s.at[1] = Math.round(v))} />
              <button class="mini" class:busy={picking === i} title="Pick on screen" onclick={() => pickInto(i)}>
                {picking === i ? "click anywhere…" : "pick"}
              </button>
              <button class="mini" title="Click wherever the cursor is instead" onclick={() => (s.at = null)}>any spot</button>
            {:else}
              <span class="soft">wherever the cursor is</span>
              <button class="mini" class:busy={picking === i} onclick={() => pickInto(i)}>{picking === i ? "click anywhere…" : "set spot"}</button>
            {/if}
            {#if s.action === "tap"}
              <span class="soft">for</span> <InlineNumber value={s.holdMs} onchange={(v) => (s.holdMs = v)} /> <span class="soft">ms</span>
            {/if}
          {:else if s.type === "move"}
            <span>Move mouse to</span>
            <InlineNumber value={s.x} min={-1e5} onchange={(v) => (s.x = Math.round(v))} />
            <InlineNumber value={s.y} min={-1e5} onchange={(v) => (s.y = Math.round(v))} />
            <button class="mini" class:busy={picking === i} onclick={() => pickInto(i)}>{picking === i ? "click anywhere…" : "pick"}</button>
          {:else if s.type === "path"}
            <span>Follow recorded mouse movement</span>
            <span class="soft">· {s.points.length} points · {formatMs(s.points.at(-1)?.t ?? 0)}</span>
          {:else if s.type === "scroll"}
            <span>Scroll</span>
            <button class="chip" onclick={() => (s.amount = -s.amount)}>{s.amount > 0 ? "up" : "down"}</button>
            <InlineNumber
              value={Math.abs(s.amount) / 120}
              min={0.1}
              onchange={(v) => (s.amount = Math.sign(s.amount || -1) * Math.round(v * 120))}
            />
            <span class="soft">{Math.abs(s.amount) === 120 ? "notch" : "notches"}</span>
          {:else if s.type === "text"}
            <span>Type</span>
            <input class="text-in" type="text" bind:value={s.text} placeholder="text to type" style:width="{Math.max(6, s.text.length + 2)}ch" />
          {:else if s.type === "repeat"}
            <span>Repeat</span>
            {#if s.times === 0}
              <button class="chip" onclick={() => (s.times = 3)}>forever</button>
            {:else}
              <InlineNumber value={s.times} min={1} onchange={(v) => (s.times = Math.round(v))} />
              <span class="soft">times</span>
              <button class="mini" onclick={() => (s.times = 0)}>forever</button>
            {/if}
            <span class="soft">· {formatMs(durationMs(s.steps))} each</span>
          {:else if s.type === "waitFor"}
            <span>Wait until</span>
            <ConditionEditor bind:condition={s.condition} />
            <span class="soft">in</span>
            <RegionChip bind:region={s.region} />
            <span class="soft">· give up after</span>
            {#if s.timeoutMs > 0}
              <InlineNumber value={s.timeoutMs / 1000} min={0.1} onchange={(v) => (s.timeoutMs = Math.round(v * 1000))} />
              <span class="soft">s and</span>
              <button class="chip" onclick={() => (s.onTimeout = s.onTimeout === "stop" ? "continue" : "stop")}>
                {s.onTimeout === "stop" ? "stop the macro" : "carry on"}
              </button>
              <button class="mini" onclick={() => (s.timeoutMs = 0)}>never</button>
            {:else}
              <button class="chip" onclick={() => (s.timeoutMs = 30000)}>never</button>
            {/if}
            <button class="mini" class:busy={viewing === s} onclick={() => (viewing = viewing === s ? null : s)}>
              {viewing === s ? "hide live view" : "live view"}
            </button>
          {/if}
        </div>
        <span class="tools">
          <button title="Move up" onclick={() => move(i, -1)} disabled={i === 0}>↑</button>
          <button title="Move down" onclick={() => move(i, 1)} disabled={i === steps.length - 1}>↓</button>
          <button title="Duplicate" onclick={() => steps.splice(i + 1, 0, structuredClone($state.snapshot(s)))}>⧉</button>
          <button title="Delete" class="del" onclick={() => steps.splice(i, 1)}>✕</button>
        </span>
      </li>
      {#if s.type === "repeat"}
        <li class="nest">
          <StepList steps={s.steps} depth={depth + 1} />
        </li>
      {:else if s.type === "waitFor" && viewing === s}
        <li class="live-view">
          <WatchViewer
            region={s.region}
            condition={s.condition}
            onpickcolor={s.condition.kind === "colorAt"
              ? (dx, dy, rgb) => {
                  if (s.condition.kind === "colorAt") Object.assign(s.condition, { dx, dy, rgb });
                }
              : undefined}
          />
        </li>
      {/if}
    {/if}
  {/each}

  <li class="add-row">
    <Popover width={330}>
      {#snippet trigger({ toggle, open })}
        <button class="add" class:open onclick={toggle}>
          <span class="plus">+</span>
          {depth > 0 ? "Add step to loop" : "Add step"}
        </button>
      {/snippet}
      {#snippet children({ close })}
        <div class="types">
          {#each STEP_TYPES.filter((t) => depth < 2 || t.type !== "repeat") as t}
            <button class="type" onclick={() => add(t.type, close)}>
              <StepIcon type={t.type} size={30} />
              <span><b>{t.label}</b><small>{t.hint}</small></span>
            </button>
          {/each}
        </div>
      {/snippet}
    </Popover>
  </li>
</ol>

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .nested {
    margin: 2px 0 6px 26px;
    padding-left: 18px;
    border-left: 2px solid rgba(61, 242, 176, 0.25);
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 50px;
    padding: 8px 12px;
    border-radius: 14px;
    background: rgba(255, 255, 255, 0.035);
    border: 1px solid var(--stroke);
    transition:
      border-color 250ms var(--ease-smooth-out),
      background 250ms var(--ease-smooth-out),
      box-shadow 250ms var(--ease-smooth-out);
  }
  .row:hover {
    background: rgba(255, 255, 255, 0.055);
  }
  /* Actions with no wait between them still need breathing room. */
  .row + .row,
  .nest + .row,
  .row + .nest {
    margin-top: 6px;
  }
  .row.active {
    border-color: rgba(61, 242, 176, 0.55);
    background: rgba(61, 242, 176, 0.07);
    box-shadow: 0 0 24px -8px rgba(61, 242, 176, 0.6);
  }
  .n {
    width: 16px;
    text-align: right;
    font-size: 11px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .say {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-weight: 550;
  }
  .soft {
    color: var(--text-3);
    font-weight: 500;
  }
  .chip {
    font-weight: 650;
    padding: 2px 10px;
    border-radius: 8px;
    color: var(--text);
    background: rgba(139, 108, 255, 0.16);
    box-shadow: inset 0 0 0 1px rgba(139, 108, 255, 0.35);
    transition: background var(--duration-quick) var(--ease-smooth-out);
  }
  .chip:hover,
  .chip.open {
    background: rgba(139, 108, 255, 0.28);
  }
  .keycap {
    min-width: 30px;
    font-weight: 700;
    font-size: 13px;
    padding: 3px 9px;
    border-radius: 7px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.14), rgba(255, 255, 255, 0.06));
    border: 1px solid rgba(255, 255, 255, 0.13);
    box-shadow: inset 0 -2px 0 rgba(0, 0, 0, 0.35);
  }
  .keycap.listening {
    color: var(--violet-2);
    border-color: rgba(139, 108, 255, 0.6);
    font-weight: 600;
  }
  .mini {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-3);
    padding: 2px 8px;
    border-radius: 99px;
    border: 1px solid var(--stroke);
    transition: all var(--duration-quick) var(--ease-smooth-out);
  }
  .mini:hover {
    color: var(--text);
    border-color: var(--stroke-strong);
  }
  .mini.busy {
    color: var(--violet-2);
    border-color: rgba(139, 108, 255, 0.6);
  }
  .text-in {
    font: inherit;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 8px;
  }
  .tools {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--duration-quick) var(--ease-smooth-out);
  }
  .row:hover .tools,
  .wait:hover .tools {
    opacity: 1;
  }
  .tools button {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--text-3);
  }
  .tools button:hover:not(:disabled) {
    background: var(--glass-hover);
    color: var(--text);
  }
  .tools button:disabled {
    opacity: 0.3;
  }
  .tools .del:hover {
    color: var(--red) !important;
  }

  /* Waits read as the connective tissue between actions. */
  .wait {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding-left: 40px;
  }
  .rail {
    position: absolute;
    left: 53px;
    top: 0;
    bottom: 0;
    width: 2px;
    background: linear-gradient(180deg, var(--stroke-strong), var(--stroke));
  }
  .wait-pill {
    position: relative;
    margin-left: 28px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-3);
    font-weight: 550;
  }
  .wait-pill :global(.num) {
    font-size: 12.5px;
    padding: 0 3px;
  }
  .wait.active .wait-pill {
    color: var(--mint);
  }
  .wait.active .rail {
    background: var(--mint);
    box-shadow: 0 0 10px var(--mint);
  }
  .wait .tools {
    margin-left: auto;
    padding-right: 12px;
  }
  .nest {
    list-style: none;
  }
  .live-view {
    margin: 6px 0 2px 40px;
    padding: 14px;
    border-radius: 14px;
    background: rgba(56, 214, 255, 0.04);
    border: 1px solid rgba(56, 214, 255, 0.2);
  }

  .add-row {
    padding-top: 10px;
  }
  .add {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px 8px 10px;
    border-radius: 12px;
    color: var(--text-3);
    font-weight: 600;
    border: 1px dashed var(--stroke-strong);
    transition: all var(--duration-quick) var(--ease-smooth-out);
  }
  .add:hover,
  .add.open {
    color: var(--text);
    border-color: rgba(139, 108, 255, 0.6);
    background: rgba(139, 108, 255, 0.08);
  }
  .plus {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    background: rgba(139, 108, 255, 0.2);
    color: var(--violet-2);
    font-size: 15px;
  }
  .types {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .type {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    padding: 7px 8px;
    border-radius: 10px;
  }
  .type:hover {
    background: var(--glass-hover);
  }
  .type b {
    display: block;
    font-weight: 600;
    font-size: 13.5px;
  }
  .type small {
    color: var(--text-3);
    font-size: 12px;
  }
  .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border-radius: 9px;
    font-weight: 550;
  }
  .menu-item:hover {
    background: var(--glass-hover);
  }
  .menu-item.on {
    color: var(--violet-2);
  }
  .grid3 {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .tile {
    height: 34px;
    border-radius: 9px;
    font-size: 12.5px;
    font-weight: 550;
    background: var(--glass);
    border: 1px solid var(--stroke);
  }
  .tile:hover {
    background: var(--glass-hover);
  }
  .tile.on {
    background: rgba(139, 108, 255, 0.2);
    border-color: rgba(139, 108, 255, 0.6);
  }
  .sep {
    height: 1px;
    background: var(--stroke);
    margin: 10px 0 6px;
  }
</style>
