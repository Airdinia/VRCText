<script lang="ts">
  import { Check, AlertCircle } from "lucide-svelte";

  interface Props {
    text: string;
    tone?: "info" | "error";
  }
  let { text, tone = "info" }: Props = $props();
</script>

<div class="toast" class:error={tone === "error"} role="status">
  {#if tone === "error"}
    <AlertCircle />
  {:else}
    <Check />
  {/if}
  <span>{text}</span>
</div>

<style>
  .toast {
    position: absolute;
    left: 50%;
    bottom: 90px;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--card);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    font-size: 12px;
    box-shadow: 0 8px 24px -8px rgba(0, 0, 0, 0.5);
    pointer-events: none;
    z-index: 8;
    animation: vrc-toast-in 250ms ease, vrc-fade 1800ms 400ms forwards;
  }
  .toast.error {
    color: var(--danger);
    border-color: var(--danger);
  }
  .toast :global(svg) {
    width: 12px;
    height: 12px;
    stroke-width: 1.5;
  }

  @keyframes vrc-fade {
    to {
      opacity: 0;
    }
  }
</style>
