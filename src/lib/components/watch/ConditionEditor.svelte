<script lang="ts">
  import { api, errorText, type Cmp, type Condition } from "$lib/api";
  import { toast } from "$lib/toast.svelte";
  import InlineNumber from "../ui/InlineNumber.svelte";
  import Popover from "../ui/Popover.svelte";
  import TemplateThumb from "./TemplateThumb.svelte";

  let { condition = $bindable() }: { condition: Condition } = $props();

  const KINDS: { kind: Condition["kind"]; label: string; hint: string }[] = [
    { kind: "textAppears", label: "text appears", hint: "e.g. “Ready” shows up" },
    { kind: "textGone", label: "text disappears", hint: "e.g. “Loading” goes away" },
    { kind: "number", label: "a number reaches", hint: "coins ≥ 1e25, 12.5K, 3M…" },
    { kind: "colorAt", label: "a colour shows", hint: "a pixel turns a colour" },
    { kind: "imageAppears", label: "an image appears", hint: "a button or icon shows up" },
    { kind: "changes", label: "anything changes", hint: "any pixel in the area" },
  ];

  let grabbing = $state(false);
  /** Drag a box around the button/icon to look for. */
  async function grabImage(): Promise<boolean> {
    grabbing = true;
    try {
      const template = await api.captureTemplate();
      if (!template) return false;
      const threshold = condition.kind === "imageAppears" ? condition.threshold : 0.9;
      condition = { kind: "imageAppears", template, threshold };
      return true;
    } catch (e) {
      toast(errorText(e), "error");
      return false;
    } finally {
      grabbing = false;
    }
  }

  function setKind(kind: Condition["kind"]) {
    if (kind === condition.kind) return;
    if (kind === "imageAppears") {
      grabImage();
      return;
    }
    const text = "text" in condition ? condition.text : "";
    const digitsAfter = "text" in condition ? (condition.digitsAfter ?? null) : null;
    condition =
      kind === "textAppears" || kind === "textGone"
        ? { kind, text, digitsAfter }
        : kind === "number"
          ? { kind, op: ">=", value: 100 }
          : kind === "colorAt"
            ? { kind, dx: 0, dy: 0, rgb: [255, 0, 0], tolerance: 30 }
            : { kind };
  }

  const OPS: Cmp[] = [">=", "<=", "=="];
  const OP_LABEL: Record<Cmp, string> = { ">=": "at least", "<=": "at most", "==": "exactly" };

  let label = $derived(KINDS.find((k) => k.kind === condition.kind)?.label ?? "");

  // "+N digits": e25 with +1 covers e250..e259 (from the old tool's min_after).
  const DIGIT_OPTIONS: { value: number | null; label: string; hint: string }[] = [
    { value: null, label: "Contains", hint: "Matches anywhere, even inside longer text" },
    { value: 0, label: "Exactly", hint: "e2 matches e2, but not e25" },
    { value: 1, label: "+1 digit", hint: "e25 matches e250 to e259" },
    { value: 2, label: "+2 digits", hint: "e2 matches e200 to e299" },
    { value: 3, label: "+3 digits", hint: "e2 matches e2000 to e2999" },
  ];
  function digitsLabel(n: number | null | undefined) {
    return n == null ? "contains" : n === 0 ? "exactly" : `+${n} digit${n === 1 ? "" : "s"}`;
  }
  function rangeHint(text: string, n: number | null | undefined): string {
    const first = text.split(",")[0]?.trim();
    if (!first || n == null || n === 0) return "";
    return `${first}${"0".repeat(n)}–${first}${"9".repeat(n)}`;
  }
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
    {@const c = condition}
    <Popover width={290}>
      {#snippet trigger({ toggle, open })}
        <button class="chip soft" class:open onclick={toggle} title="How strictly the text must match">
          {digitsLabel(c.digitsAfter)}{#if rangeHint(c.text, c.digitsAfter)}<span class="range">{rangeHint(c.text, c.digitsAfter)}</span>{/if}
        </button>
      {/snippet}
      {#snippet children({ close })}
        {#each DIGIT_OPTIONS as o}
          <button class="item" class:on={(c.digitsAfter ?? null) === o.value} onclick={() => ((c.digitsAfter = o.value), close())}>
            <b>{o.label}</b><small>{o.hint}</small>
          </button>
        {/each}
        <p class="tip">“e+25” is read the same as “e25”. Separate options with commas, like <b>e30, inf</b>.</p>
      {/snippet}
    </Popover>
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
  {:else if condition.kind === "imageAppears"}
    {@const c = condition}
    <TemplateThumb template={c.template} />
    <button class="chip soft" onclick={grabImage}>{grabbing ? "selecting…" : "change"}</button>
    <span class="soft">at least</span>
    <InlineNumber value={Math.round(c.threshold * 100)} min={50} max={100} onchange={(v) => (c.threshold = v / 100)} />
    <span class="soft">% alike</span>
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
  .range {
    margin-left: 6px;
    font-weight: 600;
    color: var(--cyan);
  }
  .tip {
    font-size: 12px;
    color: var(--text-3);
    padding: 8px 10px 2px;
    line-height: 1.45;
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
