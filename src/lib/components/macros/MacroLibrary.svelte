<script lang="ts">
  import { api, errorText, type Settings } from "$lib/api";
  import { activity, upsertMacro } from "$lib/activity.svelte";
  import { toast } from "$lib/toast.svelte";
  import MacrosEmpty from "../MacrosEmpty.svelte";
  import MacroCard from "./MacroCard.svelte";

  let { settings = $bindable(), onopen }: { settings: Settings; onopen: (id: string) => void } = $props();

  async function record() {
    try {
      if (activity.recording) await api.recordingStop();
      else await api.recordingStart();
    } catch (e) {
      toast(errorText(e), "error");
    }
  }


  let fileInput: HTMLInputElement;
  async function importFiles(e: Event) {
    const files = [...((e.currentTarget as HTMLInputElement).files ?? [])];
    (e.currentTarget as HTMLInputElement).value = "";
    if (files.length === 0) return;
    try {
      const pairs = await Promise.all(files.map(async (f) => [f.name, await f.text()] as [string, string]));
      const imported = await api.importMacros(pairs);
      imported.reverse().forEach(upsertMacro);
      toast(imported.length === 1 ? `Imported “${imported[0].name}”` : `Imported ${imported.length} macros`);
    } catch (err) {
      toast(errorText(err), "error", 7000);
    }
  }

  async function blank() {
    const saved = await api.saveMacro({ id: "", name: "New macro", steps: [], speed: 1, repeat: 1, hotkey: null, updatedAt: 0 });
    upsertMacro(saved);
    onopen(saved.id);
  }
</script>

<section class="lib">
  <div class="head">
    <div>
      <h2>Macros</h2>
      <p>Record what you do once, replay it forever.</p>
    </div>
    <div class="actions">
      <button class="btn" onclick={() => fileInput.click()} title="Import macros from the old hesytools (v4 .json files)">Import</button>
      <input bind:this={fileInput} type="file" accept=".json,application/json" multiple hidden onchange={importFiles} />
      <button class="btn" onclick={blank}>New</button>
      <button class="btn rec" class:on={activity.recording} onclick={record}>
        <span class="dot"></span>{activity.recording ? "Stop recording" : "Record"}
        {#if settings.recordHotkey && !activity.recording}<kbd>{settings.recordHotkey}</kbd>{/if}
      </button>
    </div>
  </div>


  {#if activity.macros.length === 0 && !activity.recording}
    <MacrosEmpty />
  {:else}
    <div class="grid">
      {#each activity.macros as m, i (m.id)}
        <MacroCard {m} index={i} onopen={() => onopen(m.id)} />
      {/each}
    </div>
  {/if}
</section>

<style>
  .lib {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 0 6px;
  }
  h2 {
    font-size: 19px;
    letter-spacing: -0.02em;
  }
  .head p {
    color: var(--text-3);
    font-size: 13px;
    margin-top: 2px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rec .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--red);
    box-shadow: 0 0 8px var(--red);
  }
  .rec.on {
    border-color: rgba(255, 92, 112, 0.6);
    background: rgba(255, 92, 112, 0.14);
  }
  .rec.on .dot {
    border-radius: 2px;
  }
  .rec kbd {
    font-family: inherit;
    font-size: 11px;
    font-weight: 650;
    color: var(--text-3);
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid var(--stroke);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
</style>
