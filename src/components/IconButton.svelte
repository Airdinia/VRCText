<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    active?: boolean;
    onclick?: () => void;
    children?: Snippet;
  }

  let { label, active = false, onclick, children }: Props = $props();
</script>

<button
  type="button"
  class="ib"
  class:active
  title={label}
  aria-label={label}
  onclick={onclick}
>
  {#if children}{@render children()}{/if}
</button>

<style>
  .ib {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--r-sm);
    color: var(--text-muted);
    background: transparent;
    transition: background 120ms, color 120ms;
  }
  .ib:hover {
    background: var(--card-hover);
    color: var(--text);
  }
  .ib.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .ib :global(svg) {
    width: 14px;
    height: 14px;
    stroke-width: 1.5;
  }
</style>
