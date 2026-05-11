<script lang="ts">
  import HistoryRow from "./HistoryRow.svelte";
  import { clearHistory } from "../lib/ipc";
  import { store } from "../lib/stores.svelte";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    onResendStatus: (text: string) => void;
  }
  let { onResendStatus }: Props = $props();

  // Two-step delete: first click arms, the [CONFIRM] label fills with red
  // over ~2.2s and reverts if no second click lands.
  let confirming = $state(false);
  let confirmStart = $state(0);
  let confirmTimer: ReturnType<typeof setInterval> | null = null;
  let confirmProgress = $state(0);

  const CONFIRM_WINDOW_MS = 2200;

  function arm() {
    confirming = true;
    confirmStart = performance.now();
    confirmProgress = 0;
    if (confirmTimer !== null) clearInterval(confirmTimer);
    confirmTimer = setInterval(() => {
      const p = Math.min(1, (performance.now() - confirmStart) / CONFIRM_WINDOW_MS);
      confirmProgress = p;
      if (p >= 1) {
        confirming = false;
        confirmProgress = 0;
        if (confirmTimer !== null) {
          clearInterval(confirmTimer);
          confirmTimer = null;
        }
      }
    }, 30);
  }

  async function onClear() {
    if (!confirming) {
      arm();
      return;
    }
    confirming = false;
    confirmProgress = 0;
    if (confirmTimer !== null) {
      clearInterval(confirmTimer);
      confirmTimer = null;
    }
    await clearHistory();
    onResendStatus(t("clearedToast"));
  }

  // Newest first — deque is oldest-first on disk, so we reverse for display.
  // Index passed to HistoryRow stays the deque index.
  const reversed = $derived(
    store.history.map((e, i) => ({ entry: e, index: i })).reverse(),
  );
</script>

<section class="list">
  <div class="bar">
    <span class="label">
      <span class="prefix">//</span>
      {t("historyHeader")} ·
      <span class="count">{store.history.length}</span>
    </span>
    {#if store.history.length > 0}
      <button
        type="button"
        class="clear"
        class:armed={confirming}
        onclick={onClear}
        title={confirming ? t("confirmClearTitle") : t("clearTitle")}
      >
        {#if confirming}
          <span class="fill" style:--p="{(confirmProgress * 100).toFixed(0)}%"></span>
        {/if}
        <span class="bracket">{confirming ? t("confirmClear") : t("clear")}</span>
      </button>
    {/if}
  </div>

  <div class="rows">
    {#if store.history.length === 0}
      <div class="empty">
        <div class="empty-mark">{">"}_</div>
        <div class="empty-line">{t("emptyHistory")}</div>
        <div class="empty-hint">{t("emptyHint")}</div>
      </div>
    {:else}
      {#each reversed as { entry, index } (index + ":" + entry.ts)}
        <HistoryRow {entry} {index} onResend={onResendStatus} />
      {/each}
    {/if}
  </div>
</section>

<style>
  .list {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .bar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px 6px;
    color: var(--text-faint);
    font-size: 10.5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .prefix {
    color: var(--accent);
    opacity: 0.7;
  }
  .count {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .clear {
    position: relative;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    color: var(--text-faint);
    font-size: 10.5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    overflow: hidden;
    transition: color 120ms;
  }
  .clear:hover {
    color: var(--text);
  }
  .clear.armed {
    color: var(--danger);
  }
  .clear .fill {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      to right,
      color-mix(in oklab, var(--danger) 16%, transparent) var(--p),
      transparent var(--p)
    );
    pointer-events: none;
  }
  .clear .bracket {
    position: relative;
  }
  .rows {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 20px 0;
    color: var(--text-faint);
  }
  .empty-mark {
    color: var(--accent);
    font-size: 16px;
    letter-spacing: 0.1em;
  }
  .empty-line {
    font-size: 12.5px;
    color: var(--text-muted);
  }
  .empty-hint {
    font-size: 11px;
    color: var(--text-faint);
  }
</style>
