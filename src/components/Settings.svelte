<script lang="ts">
  import { X, AlertCircle } from "lucide-svelte";
  import { savePartialConfig, type Lang } from "../lib/ipc";
  import { store } from "../lib/stores.svelte";
  import { t, setLanguage } from "../lib/i18n.svelte";
  import TtsSection from "./TtsSection.svelte";

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  const cfg = $derived(store.config);

  let ipDraft = $state("");
  let portDraft = $state("");

  // Sync drafts from config only when the backend value itself changed.
  // `config-changed` fires for every config mutation (pin, bell, TTS
  // toggles…) with a fresh object — resetting drafts unconditionally would
  // wipe an in-progress edit whenever the user touches another control.
  let lastCfgIp: string | null = null;
  let lastCfgPort: number | null = null;
  $effect(() => {
    if (!cfg) return;
    if (cfg.ip !== lastCfgIp) {
      lastCfgIp = cfg.ip;
      ipDraft = cfg.ip;
    }
    if (cfg.port !== lastCfgPort) {
      lastCfgPort = cfg.port;
      portDraft = String(cfg.port);
    }
  });

  const portValid = $derived(
    /^\d{1,5}$/.test(portDraft) &&
      +portDraft > 0 &&
      +portDraft < 65536,
  );
  const ipValid = $derived(/^[a-zA-Z0-9.-]+$/.test(ipDraft.trim()));

  async function commitIp() {
    const v = ipDraft.trim();
    if (!ipValid || v === cfg?.ip) return;
    await savePartialConfig({ ip: v });
  }

  async function commitPort() {
    if (!portValid) return;
    const n = +portDraft;
    if (n === cfg?.port) return;
    await savePartialConfig({ port: n });
  }

  async function pickLang(lang: Lang) {
    if (cfg?.language === lang) return;
    await setLanguage(lang);
  }
</script>

<button class="scrim" type="button" onclick={onClose} aria-label={t("closeSettings")}></button>
<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
<aside class="drawer" role="dialog" aria-label={t("settings")}>
  <header class="dh">
    <h2>{t("settings")}</h2>
    <button
      class="close"
      type="button"
      onclick={onClose}
      title={t("close")}
      aria-label={t("close")}
    >
      <X />
    </button>
  </header>

  <div class="body">
    <section class="section">
      <!-- Hardcoded bilingual so an English user looking at the default
           zh UI can still recognize the language switch. -->
      <div class="section-label">语言 / Language</div>
      <div class="seg" role="radiogroup" aria-label={t("language")}>
        <button
          type="button"
          class="seg-btn"
          class:on={cfg?.language !== "en"}
          onclick={() => pickLang("zh")}
        >
          中文
        </button>
        <button
          type="button"
          class="seg-btn"
          class:on={cfg?.language === "en"}
          onclick={() => pickLang("en")}
        >
          English
        </button>
      </div>
    </section>

    <section class="section">
      <div class="section-label">{t("oscTarget")}</div>
      <div class="rows">
        <label class="row">
          <span class="row-label">{t("ipAddress")}</span>
          <input
            type="text"
            bind:value={ipDraft}
            onblur={commitIp}
            onkeydown={(e) => e.key === "Enter" && commitIp()}
            spellcheck="false"
          />
        </label>
        <label class="row">
          <span class="row-label">{t("port")}</span>
          <input
            type="text"
            class:error={!portValid}
            bind:value={portDraft}
            onblur={commitPort}
            onkeydown={(e) => e.key === "Enter" && commitPort()}
            inputmode="numeric"
          />
          {#if !portValid}
            <div class="err">
              <AlertCircle /> {t("portError")}
            </div>
          {/if}
        </label>
      </div>
    </section>

    <TtsSection />
  </div>
</aside>

<style>
  /* Scrim — clickable backdrop that dismisses the drawer */
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 5;
    background: rgba(0, 0, 0, 0.45);
    border: 0;
    padding: 0;
    cursor: default;
    animation: vrc-scrim-in 180ms ease;
  }
  @keyframes vrc-scrim-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 76%;
    z-index: 6;
    background: var(--bg);
    border-left: 1px solid var(--border);
    box-shadow: -12px 0 32px -8px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    animation: vrc-drawer-in 220ms cubic-bezier(0.22, 0.61, 0.36, 1);
  }
  @keyframes vrc-drawer-in {
    from {
      transform: translateX(100%);
    }
    to {
      transform: translateX(0);
    }
  }

  .dh {
    flex-shrink: 0;
    padding: 10px 10px 10px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h2 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .close {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--r-sm);
    color: var(--text-muted);
    background: transparent;
    transition: background 120ms, color 120ms;
  }
  .close:hover {
    background: var(--card-hover);
    color: var(--text);
  }
  .close :global(svg) {
    width: 14px;
    height: 14px;
    stroke-width: 1.5;
  }

  .body {
    flex: 1;
    overflow: auto;
    padding: 0 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    color: var(--text);
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  .section-label {
    color: var(--text-faint);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-weight: 600;
    margin-bottom: 8px;
    padding-top: 4px;
  }
  .rows {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row-label {
    font-size: 11.5px;
    opacity: 0.7;
  }
  input[type="text"] {
    appearance: none;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 5px 8px;
    outline: none;
    transition: border-color 120ms;
  }
  input[type="text"]:focus {
    border-color: var(--accent-ring);
  }
  input[type="text"].error {
    border-color: var(--danger);
  }
  .err {
    color: var(--danger);
    font-size: 10.5px;
    margin-top: 4px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .err :global(svg) {
    width: 10px;
    height: 10px;
    stroke-width: 1.5;
  }

  /* Language segmented switch — mirrors the engine picker in TtsSection */
  .seg {
    display: flex;
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 2px;
    gap: 2px;
  }
  .seg-btn {
    flex: 1;
    padding: 4px 8px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-muted);
    background: transparent;
    border-radius: var(--r-sm);
    transition: background 120ms, color 120ms;
  }
  .seg-btn.on {
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
