<script lang="ts">
  import type { StepType } from "$lib/api";
  import { ICON, TINT } from "$lib/steps";
  let { type, size = 28 }: { type: StepType; size?: number } = $props();
</script>

<span class="ic {TINT[type]}" style:width="{size}px" style:height="{size}px">
  <svg width={size * 0.5} height={size * 0.5} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
    <path d={ICON[type]} />
  </svg>
</span>

<style>
  .ic {
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
  }
  .violet {
    color: var(--violet-2);
    background: rgba(139, 108, 255, 0.18);
  }
  .pink {
    color: var(--pink);
    background: rgba(255, 110, 199, 0.15);
  }
  .gray {
    color: var(--text-2);
    background: rgba(255, 255, 255, 0.08);
  }
  .blue {
    color: #8fb0ff;
    background: rgba(91, 140, 255, 0.16);
  }
  .cyan {
    color: var(--cyan);
    background: rgba(56, 214, 255, 0.14);
  }
  .amber {
    color: var(--amber);
    background: rgba(255, 191, 92, 0.14);
  }
  .mint {
    color: var(--mint);
    background: rgba(61, 242, 176, 0.13);
  }
</style>
