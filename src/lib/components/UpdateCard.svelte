<script lang="ts">
  // "hesytools got an update": shown when GitHub has a newer version.
  import { installUpdate, update } from "$lib/update.svelte";

  let showNotes = $state(false);
  let notes = $derived(
    (update.info?.notes ?? "")
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => l.replace(/^[-*]\s*/, "")),
  );
  let percent = $derived(update.total ? Math.min(100, Math.round((update.downloaded / update.total) * 100)) : null);
  const mb = (b: number) => (b / 1048576).toFixed(1);
  let busy = $derived(update.phase === "downloading" || update.phase === "installing");
</script>

{#if update.info && update.phase !== "idle" && (!update.dismissed || busy)}
  <div class="card" role="dialog" aria-label="Update available">
    <div class="head">
      <img src="/logo.png" alt="" />
      <div>
        <b>hesytools got an update</b>
        <small>Version {update.info.version} · you have {update.info.currentVersion}</small>
      </div>
    </div>

    {#if update.phase === "available"}
      {#if notes.length}
        <button class="whats-new" onclick={() => (showNotes = !showNotes)}>
          {showNotes ? "Hide" : "What's new"}
          <svg class:open={showNotes} width="11" height="11" viewBox="0 0 12 12"
            ><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg
          >
        </button>
        {#if showNotes}
          <ul class="notes">
            {#each notes as n}<li>{n}</li>{/each}
          </ul>
        {/if}
      {/if}
      <p class="keep">Your macros, watches and settings stay as they are.</p>
      <div class="actions">
        <button class="btn" onclick={() => (update.dismissed = true)}>Later</button>
        <button class="btn primary" onclick={installUpdate}>Update now</button>
      </div>
    {:else}
      <div class="progress">
        <div class="bar"><span style:width="{update.phase === 'installing' ? 100 : (percent ?? 8)}%" class:indeterminate={percent === null && update.phase === "downloading"}></span></div>
        <small>
          {#if update.phase === "installing"}
            Installing… hesytools will reopen in a moment
          {:else if percent !== null}
            Downloading… {percent}% ({mb(update.downloaded)} of {mb(update.total ?? 0)} MB)
          {:else}
            Downloading…
          {/if}
        </small>
      </div>
    {/if}
  </div>
{/if}

<style>
  .card {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 70;
    width: 340px;
    padding: 16px;
    border-radius: 18px;
    background: #1a1824;
    border: 1px solid rgba(139, 108, 255, 0.45);
    box-shadow:
      0 24px 60px -16px rgba(0, 0, 0, 0.8),
      0 0 40px -18px rgba(139, 108, 255, 0.8);
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: rise 400ms var(--ease-smooth-out) both;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .head img {
    width: 38px;
    height: 38px;
  }
  .head b {
    display: block;
    font-weight: 650;
    font-size: 14.5px;
  }
  .head small {
    color: var(--text-3);
    font-size: 12px;
  }
  .whats-new {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--violet-2);
  }
  .whats-new svg {
    transition: transform 250ms var(--ease-smooth-out);
  }
  .whats-new svg.open {
    transform: rotate(180deg);
  }
  .notes {
    margin: 0;
    padding: 10px 12px 10px 28px;
    max-height: 160px;
    overflow-y: auto;
    border-radius: 12px;
    background: rgba(0, 0, 0, 0.25);
    border: 1px solid var(--stroke);
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.5;
  }
  .keep {
    font-size: 12px;
    color: var(--text-3);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .bar {
    height: 6px;
    border-radius: 99px;
    background: rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 99px;
    background: linear-gradient(90deg, var(--violet), var(--violet-2));
    transition: width 200ms var(--ease-smooth-out);
  }
  .bar span.indeterminate {
    width: 30% !important;
    animation: slide 1.2s ease-in-out infinite;
  }
  .progress small {
    font-size: 12px;
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(14px) scale(0.98);
    }
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
