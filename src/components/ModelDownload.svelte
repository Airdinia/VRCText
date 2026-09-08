<script lang="ts">
  import { Check, Download, Trash2, AlertCircle } from "lucide-svelte";
  import { downloadPack, deleteModels, type ModelKindStr } from "../lib/ipc";
  import { store, pushToast } from "../lib/stores.svelte";
  import { t, tf, maybeT, type Key } from "../lib/i18n.svelte";

  // Two-step delete confirm
  let deleteConfirmAt = $state(0);
  const deleteArmed = $derived(performance.now() < deleteConfirmAt);

  // Pack names live in the i18n dict so they flip with the UI language.
  // Sizes are units, not text — left as-is.
  const PACKS: { kind: ModelKindStr; nameKey: Key; size: string }[] = [
    { kind: "matcha", nameKey: "matchaName", size: "~129 MB" },
    { kind: "kokoro", nameKey: "kokoroName", size: "~365 MB" },
  ];

  const downloading = $derived(store.download !== null);

  function isInstalled(k: ModelKindStr) {
    return store.installedPacks.includes(k);
  }

  function progressFor(k: ModelKindStr): number {
    return store.download?.kind === k ? store.download.progress : 0;
  }

  function stageFor(k: ModelKindStr): string {
    return store.download?.kind === k ? store.download.status : "";
  }

  function statusFor(k: ModelKindStr): "installed" | "downloading" | "not" {
    if (store.download?.kind === k) return "downloading";
    if (isInstalled(k)) return "installed";
    return "not";
  }

  async function startDownload(k: ModelKindStr) {
    if (downloading) return;
    // Clear any prior error immediately so the retry feels responsive —
    // the backend's first download-progress event also clears it, but
    // that arrives a moment later than the user click.
    store.downloadError = null;
    try {
      await downloadPack(k);
    } catch (e) {
      pushToast(maybeT(String(e)), "error");
    }
  }

  async function onDelete() {
    if (downloading) return;
    if (!deleteArmed) {
      deleteConfirmAt = performance.now() + 2000;
      setTimeout(() => {
        if (performance.now() >= deleteConfirmAt) deleteConfirmAt = 0;
      }, 2050);
      return;
    }
    deleteConfirmAt = 0;
    try {
      await deleteModels();
      store.installedPacks = [];
    } catch (e) {
      pushToast(maybeT(String(e)), "error");
    }
  }
</script>

<div class="block">
  {#each PACKS as pack (pack.kind)}
    {@const status = statusFor(pack.kind)}
    <div class="card">
      <div class="head">
        <div class="meta">
          <div class="name">{t(pack.nameKey)}</div>
          <div class="size">{pack.size}</div>
        </div>
        {#if status === "installed"}
          <span class="ok"><Check /> {t("installed")}</span>
        {:else if status === "downloading"}
          <span class="pct">{(progressFor(pack.kind) * 100).toFixed(0)}%</span>
        {:else}
          <button
            type="button"
            class="dl"
            disabled={downloading}
            onclick={() => startDownload(pack.kind)}
          >
            <Download /> {t("download")}
          </button>
        {/if}
      </div>
      {#if status === "downloading"}
        <div class="stage">{maybeT(stageFor(pack.kind))}</div>
        <div class="bar">
          <div
            class="bar-fill"
            style:width="{(progressFor(pack.kind) * 100).toFixed(1)}%"
          ></div>
        </div>
      {/if}
    </div>
  {/each}

  {#if store.downloadError}
    <div class="err-block">
      <div class="err">
        <AlertCircle /> {tf("downloadFailed", { message: maybeT(store.downloadError.message) })}
      </div>
      <div class="err-actions">
        <button
          type="button"
          class="retry"
          disabled={downloading}
          onclick={() => startDownload(store.downloadError!.kind)}
        >
          {t("retry")}
        </button>
        <button
          type="button"
          class="dismiss"
          onclick={() => (store.downloadError = null)}
        >
          {t("dismiss")}
        </button>
      </div>
    </div>
  {/if}

  {#if store.installedPacks.length > 0}
    <button
      type="button"
      class="del"
      class:armed={deleteArmed}
      disabled={downloading}
      onclick={onDelete}
    >
      <Trash2 />
      {deleteArmed ? t("confirmDeleteAll") : t("deleteAllModels")}
    </button>
  {/if}
</div>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: relative;
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .meta {
    display: flex;
    flex-direction: column;
  }
  .name {
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }
  .size {
    color: var(--text-faint);
    font-size: 10.5px;
  }
  .ok {
    color: var(--ok);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11.5px;
  }
  .ok :global(svg) {
    width: 11px;
    height: 11px;
    stroke-width: 1.5;
  }
  .pct {
    color: var(--accent);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .dl {
    background: var(--accent);
    color: var(--accent-text);
    padding: 3px 8px;
    border-radius: var(--r-sm);
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    transition: opacity 120ms;
  }
  .dl:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .dl :global(svg) {
    width: 10px;
    height: 10px;
    stroke-width: 1.5;
  }
  .stage {
    font-size: 10.5px;
    color: var(--text-muted);
    margin-top: 2px;
  }
  .bar {
    height: 3px;
    background: var(--accent-soft);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 4px;
  }
  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 200ms;
  }
  .err-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .err {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    color: var(--danger);
    background: color-mix(in oklab, var(--danger) 12%, transparent);
    border-radius: var(--r-sm);
    font-size: 11px;
  }
  .err :global(svg) {
    width: 10px;
    height: 10px;
    stroke-width: 1.5;
  }
  .err-actions {
    display: flex;
    gap: 6px;
    align-self: flex-start;
  }
  .retry,
  .dismiss {
    padding: 3px 10px;
    border-radius: var(--r-sm);
    font-size: 11px;
    transition: background 120ms, color 120ms, border-color 120ms;
    border: 1px solid var(--border);
  }
  .retry {
    background: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .retry:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .dismiss {
    background: transparent;
    color: var(--text-muted);
  }
  .dismiss:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .del {
    align-self: flex-start;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--danger);
    font-size: 11.5px;
    padding: 4px 10px;
    border-radius: var(--r-sm);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: background 120ms;
  }
  .del.armed {
    background: var(--danger);
    color: #000;
    border-color: var(--danger);
  }
  .del:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .del :global(svg) {
    width: 10px;
    height: 10px;
    stroke-width: 1.5;
  }
</style>
