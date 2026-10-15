<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, errorText, type Macro } from "$lib/api";
  import { activity } from "$lib/activity.svelte";
  import { macroSummary, preview } from "$lib/steps";
  import { toast } from "$lib/toast.svelte";
  import StepIcon from "../ui/StepIcon.svelte";

  let { m, onopen, index = 0 }: { m: Macro; onopen: () => void; index?: number } = $props();

  let playing = $derived(activity.playingId === m.id);
  let progress = $derived(playing && m.steps.length ? (activity.step + 1) / m.steps.length : 0);
  let chips = $derived(preview(m.steps));

  let countdown = $state(0);
  let timer: ReturnType<typeof setInterval> | undefined;
  onDestroy(() => clearInterval(timer));

  async function playStop(e: MouseEvent) {
    e.stopPropagation();
    if (countdown > 0) {
      clearInterval(timer);
      countdown = 0;
    } else if (playing) {
      await api.stopMacro();
    } else {
      countdown = 3;
      timer = setInterval(async () => {
        if (countdown <= 1) {
          clearInterval(timer);
          countdown = 0;
          await api.playMacro(m.id).catch((err) => toast(errorText(err), "error"));
        } else countdown -= 1;
      }, 1000);
    }
  }
</script>

<div
  class="card glass"
  class:playing
  style:--i={index}
  role="button"
  tabindex="0"
  onclick={onopen}
  onkeydown={(e) => e.key === "Enter" && onopen()}
>
  <div class="text">
    <h3>{m.name}</h3>
    <p>{macroSummary(m)}</p>
  </div>

  <div class="chips">
    {#each chips as c}
      <span class="c"><StepIcon type={c.type} size={20} />{c.text}</span>
    {/each}
    {#if m.steps.filter((s) => s.type !== "wait").length > chips.length}
      <span class="more">…</span>
    {/if}
  </div>

  <div class="foot">
    {#if m.hotkey}
      <span class="hk">{#each m.hotkey.split("+") as k}<kbd>{k}</kbd>{/each}</span>
    {:else}
      <span class="nohk">No hotkey</span>
    {/if}
    <button class="go" class:playing class:counting={countdown > 0} onclick={playStop} aria-label={playing ? "Stop" : "Play"}>
      {#if countdown > 0}
        <b>{countdown}</b>
      {:else if playing}
        <svg width="14" height="14" viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="14" rx="3" fill="currentColor" /></svg>
      {:else}
        <svg width="15" height="15" viewBox="0 0 24 24"><path d="M8 5.5v13a1 1 0 0 0 1.5.87l11-6.5a1 1 0 0 0 0-1.74l-11-6.5A1 1 0 0 0 8 5.5z" fill="currentColor" /></svg>
      {/if}
    </button>
  </div>

  <span class="bar" style:transform="scaleX({progress})"></span>
</div>

<style>
  .card {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 18px 16px;
    cursor: pointer;
    text-align: left;
    animation: rise 400ms var(--ease-smooth-out) both;
    animation-delay: calc(var(--i) * 40ms);
    transition:
      transform 250ms var(--ease-smooth-out),
      border-color 300ms var(--ease-smooth-out),
      box-shadow 300ms var(--ease-smooth-out);
  }
  .card:hover {
    transform: translateY(-2px);
    border-color: var(--stroke-strong);
  }
  .card.playing {
    border-color: rgba(61, 242, 176, 0.45);
    box-shadow: 0 0 40px -16px rgba(61, 242, 176, 0.6);
  }
  h3 {
    font-size: 15.5px;
    font-weight: 650;
    letter-spacing: -0.015em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .text p {
    font-size: 12.5px;
    color: var(--text-3);
    margin-top: 3px;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-height: 28px;
  }
  .c {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    padding: 3px 9px 3px 4px;
    border-radius: 9px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid var(--stroke);
  }
  .c :global(.ic) {
    border-radius: 6px;
  }
  .more {
    color: var(--text-3);
    align-self: center;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .hk {
    display: inline-flex;
    gap: 3px;
  }
  kbd {
    font-family: inherit;
    font-size: 11.5px;
    font-weight: 650;
    padding: 2px 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.12);
    box-shadow: inset 0 -1.5px 0 rgba(0, 0, 0, 0.35);
  }
  .nohk {
    font-size: 12px;
    color: var(--text-3);
  }
  .go {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: white;
    background: linear-gradient(180deg, #9a7fff, #7050f2);
    box-shadow: 0 6px 18px -6px rgba(139, 108, 255, 0.8);
    transition:
      transform 300ms var(--ease-bounce),
      background 300ms var(--ease-in-out);
  }
  .go svg {
    margin-left: 2px;
  }
  .go:hover {
    transform: scale(1.1);
  }
  .go.playing {
    background: linear-gradient(180deg, #3df2b0, #14b886);
    color: #052e22;
    box-shadow: 0 6px 18px -6px rgba(61, 242, 176, 0.7);
  }
  .go.playing svg {
    margin: 0;
  }
  .go.counting {
    background: linear-gradient(180deg, #ffcf7a, #f59f2a);
    color: #3b2300;
  }
  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--mint), var(--cyan));
    transform-origin: left;
    transition: transform 200ms var(--ease-smooth-out);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
</style>
