<script lang="ts">
  import type { Cmp, Condition } from "$lib/api";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";

  let { condition = $bindable() }: { condition: Condition } = $props();

  const KINDS: { kind: Condition["kind"]; label: string; hint: string }[] = [
    { kind: "textAppears", label: "text appears", hint: "e.g. “Ready” shows up" },
    { kind: "textGone", label: "text disappears", hint: "e.g. “Loading” goes away" },
    { kind: "number", label: "a number reaches", hint: "coins ≥ 1e25, 12.5K, 3M…" },
    { kind: "colorAt", label: "a colour shows", hint: "a pixel turns a colour" },
    { kind: "changes", label: "anything changes", hint: "any pixel in the area" },
  ];

  function setKind(kind: Condition["kind"]) {
    if (kind === condition.kind) return;
    const text = "text" in condition ? condition.text : "";
    condition =
      kind === "textAppears" || kind === "textGone"
        ? { kind, text }
        : kind === "number"
          ? { kind, op: ">=", value: 100 }
          : kind === "colorAt"
            ? { kind, dx: 0, dy: 0, rgb: [255, 0, 0], tolerance: 30 }
            : { kind };
  }

  const OPS: Cmp[] = [">=", "<=", "=="];
  const OP_LABEL: Record<Cmp, string> = { ">=": "at least", "<=": "at most", "==": "exactly" };

  let label = $derived(KINDS.find((k) => k.kind === condition.kind)?.label ?? "");
  const hex = (rgb: number[]) => "#" + rgb.map((c) => c.toString(16).padStart(2, "0")).join("");
</script>

<span class="cond">
  <Popover width={260}>
    {#snippet trigger({ toggle, open })}
      <button class="chip" class:open onclick={toggle}>{label}</button>
    {/snippet}
    {#snippet children({ close })}
      {#each KINDS as k}
        <button class="item" class:on={condition.kind === k.kind} onclick={() => (setKind(k.kind), close())}>
          <b>{k.label[0].toUpperCase() + k.label.slice(1)}</b><small>{k.hint}</small>
        </button>
      {/each}
    {/snippet}
  </Popover>

  {#if condition.kind === "textAppears" || condition.kind === "textGone"}
    <input
      class="text"
      type="text"
      bind:value={condition.text}
      placeholder="Ready, Done"
      title="Separate options with commas; any of them counts"
      style:width="{Math.max(10, condition.text.length + 3)}ch"
    />
  {:else if condition.kind === "number"}
    {@const c = condition}
    <button class="chip soft" onclick={() => (c.op = OPS[(OPS.indexOf(c.op) + 1) % OPS.length])}>{OP_LABEL[c.op]}</button>
    <InlineNumber value={c.value} min={-1e308} max={1e308} onchange={(v) => (c.value = v)} />
  {:else if condition.kind === "colorAt"}
    {@const c = condition}
    <span class="swatch" style:background={hex(c.rgb)} title={hex(c.rgb)}></span>
    <span class="soft">within</span>
    <InlineNumber value={c.tolerance} max={255} onchange={(v) => (c.tolerance = Math.round(v))} />
    <span class="soft">at pixel {c.dx}, {c.dy}</span>
  {/if}
</span>

<style>
  .cond {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip {
    font-weight: 650;
    padding: 2px 10px;
    border-radius: 8px;
    color: var(--text);
    background: rgba(56, 214, 255, 0.14);
    box-shadow: inset 0 0 0 1px rgba(56, 214, 255, 0.35);
    transition: background var(--duration-quick) var(--ease-smooth-out);
  }
  .chip:hover,
  .chip.open {
    background: rgba(56, 214, 255, 0.24);
  }
  .chip.soft {
    background: rgba(255, 255, 255, 0.07);
    box-shadow: none;
  }
  .text {
    font: inherit;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: 8px;
  }
  .soft {
    color: var(--text-3);
    font-weight: 500;
  }
  .swatch {
    width: 20px;
    height: 20px;
    border-radius: 6px;
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.25),
      0 0 10px rgba(0, 0, 0, 0.4);
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
    color: var(--cyan);
  }
</style>
