<script lang="ts">
  // On/off switch with a double-bounce thumb (transitions.dev "Toggle").
  let { checked, onchange, label }: { checked: boolean; onchange: (v: boolean) => void; label?: string } = $props();
  let init = $state(false);
</script>

<button
  class="t-toggle"
  class:is-init={init}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  data-on={checked}
  onclick={() => {
    init = true;
    onchange(!checked);
  }}
>
  <span class="t-toggle-thumb"></span>
</button>

<style>
  .t-toggle {
    --toggle-travel: 18px;
    flex: none;
    width: 42px;
    height: 24px;
    border-radius: 999px;
    padding: 3px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid var(--stroke);
    transition: background 200ms var(--ease-smooth-out);
  }
  .t-toggle[data-on="true"] {
    background: linear-gradient(180deg, #9a7fff, #7656f5);
    box-shadow: 0 0 16px -4px rgba(139, 108, 255, 0.8);
  }
  .t-toggle-thumb {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    translate: 0 0;
    will-change: translate;
  }
  .t-toggle[data-on="true"] .t-toggle-thumb {
    translate: var(--toggle-travel) 0;
  }
  .t-toggle.is-init[data-on="true"] .t-toggle-thumb {
    animation: t-toggle-on 350ms cubic-bezier(0.34, 1.35, 0.64, 1) both;
  }
  .t-toggle.is-init[data-on="false"] .t-toggle-thumb {
    animation: t-toggle-off 350ms cubic-bezier(0.34, 1.35, 0.64, 1) both;
  }
  @keyframes t-toggle-on {
    0% {
      translate: 0 0;
    }
    55% {
      translate: calc(var(--toggle-travel) + 1px) 0;
    }
    100% {
      translate: var(--toggle-travel) 0;
    }
  }
  @keyframes t-toggle-off {
    0% {
      translate: var(--toggle-travel) 0;
    }
    55% {
      translate: -1px 0;
    }
    100% {
      translate: 0 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .t-toggle-thumb {
      animation: none !important;
    }
  }
</style>
