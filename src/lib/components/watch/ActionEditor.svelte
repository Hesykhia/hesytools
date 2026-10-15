<script lang="ts">
  import { api, errorText, MOUSE_BUTTONS, type WatchAction } from "$lib/api";
  import { activity } from "$lib/activity.svelte";
  import { toast } from "$lib/toast.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";

  let { action = $bindable() }: { action: WatchAction } = $props();

  const BUTTON_WORD: Record<string, string> = {
    LeftMouse: "left",
    RightMouse: "right",
    MiddleMouse: "middle",
    Mouse4: "Mouse 4",
    Mouse5: "Mouse 5",
  };

  let label = $derived.by(() => {
    const a = action;
    switch (a.kind) {
      case "runMacro":
        return `run “${activity.macros.find((m) => m.id === a.macroId)?.name ?? "pick a macro"}”`;
      case "stopEverything":
        return "stop everything";
      case "click":
        return "click";
      case "pressKey":
        return "press a key";
      case "shutdown":
        return a.restart ? "restart the PC" : "shut down the PC";
      default:
        return "just notify me";
    }
  });

  let capturing = $state(false);
  async function captureKey() {
    if (action.kind !== "pressKey") return;
    if (capturing) return api.cancelCapture();
    capturing = true;
    try {
      const combo = await api.captureHotkey();
      if (!combo) return;
      const key = combo.split("+").pop()!;
      if (MOUSE_BUTTONS.some((b) => b.value === key)) {
        toast("That's a mouse button. Use “Click it” for that.");
        return;
      }
      action.key = key;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      capturing = false;
    }
  }

  let picking = $state(false);
  async function pickSpot() {
    if (action.kind !== "click") return;
    if (picking) return api.cancelPick();
    picking = true;
    try {
      const p = await api.pickPoint();
      if (p && action.kind === "click") action.at = { kind: "point", x: p[0], y: p[1] };
    } finally {
      picking = false;
    }
  }

  const keepTimes = () => ("times" in action ? action.times : 1);
</script>

<Popover width={290}>
  {#snippet trigger({ toggle, open })}
    <button class="chip" class:open onclick={toggle}>{label}</button>
  {/snippet}
  {#snippet children({ close })}
    {@const pick = (a: WatchAction) => ((action = a), close())}
    <button class="item" class:on={action.kind === "notify"} onclick={() => pick({ kind: "notify" })}>
      <b>Just notify me</b><small>A pop-up here (and Discord if on)</small>
    </button>
    <button
      class="item"
      class:on={action.kind === "click"}
      onclick={() => pick(action.kind === "click" ? action : { kind: "click", button: "LeftMouse", times: keepTimes(), at: { kind: "found" } })}
    >
      <b>Click it</b><small>On what it found, or a spot you pick</small>
    </button>
    <button
      class="item"
      class:on={action.kind === "pressKey"}
      onclick={() => pick(action.kind === "pressKey" ? action : { kind: "pressKey", key: "E", times: keepTimes() })}
    >
      <b>Press a key</b><small>Once or a few times</small>
    </button>
    <button class="item" class:on={action.kind === "stopEverything"} onclick={() => pick({ kind: "stopEverything" })}>
      <b>Stop everything</b><small>Like pressing the panic key</small>
    </button>
    <button class="item" class:on={action.kind === "shutdown" && !action.restart} onclick={() => pick({ kind: "shutdown", restart: false })}>
      <b>Shut down the PC</b><small>After a 1 min countdown you can cancel</small>
    </button>
    <button class="item" class:on={action.kind === "shutdown" && action.restart} onclick={() => pick({ kind: "shutdown", restart: true })}>
      <b>Restart the PC</b><small>After a 1 min countdown you can cancel</small>
    </button>
    <div class="eyebrow pad">Run a macro</div>
    {#each activity.macros as m}
      <button
        class="item"
        class:on={action.kind === "runMacro" && action.macroId === m.id}
        onclick={() => pick({ kind: "runMacro", macroId: m.id })}><b>{m.name}</b></button
      >
    {:else}
      <p class="none">No macros yet. Record one on the Macro tab.</p>
    {/each}
  {/snippet}
</Popover>

{#if action.kind === "click"}
  {@const a = action}
  <Popover width={220}>
    {#snippet trigger({ toggle, open })}
      <button class="chip soft" class:open onclick={toggle}>{BUTTON_WORD[a.button]}</button>
    {/snippet}
    {#snippet children({ close })}
      {#each MOUSE_BUTTONS as b}
        <button class="item" class:on={a.button === b.value} onclick={() => ((a.button = b.value), close())}><b>{b.label}</b></button>
      {/each}
    {/snippet}
  </Popover>
  <InlineNumber value={a.times} min={1} max={100} onchange={(v) => (a.times = Math.round(v))} />
  <span class="soft">{a.times === 1 ? "time" : "times"}</span>
  <Popover width={260}>
    {#snippet trigger({ toggle, open })}
      <button class="chip soft" class:open onclick={toggle}>
        {a.at.kind === "found" ? "on what it found" : `at ${a.at.x}, ${a.at.y}`}
      </button>
    {/snippet}
    {#snippet children({ close })}
      <button class="item" class:on={a.at.kind === "found"} onclick={() => ((a.at = { kind: "found" }), close())}>
        <b>On what it found</b><small>The image or colour it spotted; for text, the middle of the area</small>
      </button>
      <button class="item" class:on={a.at.kind === "point"} onclick={() => (close(), pickSpot())}>
        <b>{picking ? "Click anywhere… (Esc cancels)" : "A spot I pick"}</b><small>Click the spot on any monitor</small>
      </button>
    {/snippet}
  </Popover>
{:else if action.kind === "pressKey"}
  {@const a = action}
  <button class="chip soft" class:capturing onclick={captureKey} title="Click, then press the key">
    {capturing ? "Press a key…" : a.key}
  </button>
  <InlineNumber value={a.times} min={1} max={100} onchange={(v) => (a.times = Math.round(v))} />
  <span class="soft">{a.times === 1 ? "time" : "times"}</span>
{/if}

<style>
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
  .chip.soft {
    font-weight: 600;
    background: var(--glass);
    box-shadow: inset 0 0 0 1px var(--stroke);
  }
  .chip.soft:hover,
  .chip.soft.open {
    background: var(--glass-hover);
  }
  .chip.capturing {
    color: var(--violet-2);
    box-shadow: inset 0 0 0 1px rgba(139, 108, 255, 0.6);
  }
  .soft {
    color: var(--text-3);
    font-size: 13px;
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
</style>
