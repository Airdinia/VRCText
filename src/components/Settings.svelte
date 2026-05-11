<script lang="ts">
  import { X, AlertCircle } from "lucide-svelte";
  import { savePartialConfig } from "../lib/ipc";
  import { store } from "../lib/stores.svelte";
  import TtsSection from "./TtsSection.svelte";

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  const cfg = $derived(store.config);

  let ipDraft = $state("");
  let portDraft = $state("");

  $effect(() => {
    if (cfg) {
      ipDraft = cfg.ip;
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
</script>

<button class="scrim" type="button" onclick={onClose} aria-label="关闭设置"></button>
<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
<aside class="drawer" role="dialog" aria-label="设置">
  <header class="dh">
    <h2>设置</h2>
    <button
      class="close"
      type="button"
      onclick={onClose}
      title="关闭"
      aria-label="关闭"
    >
      <X />
    </button>
  </header>

  <div class="body">
    <section class="section">
      <div class="section-label">OSC 目标</div>
      <div class="rows">
        <label class="row">
          <span class="row-label">IP 地址</span>
          <input
            type="text"
            bind:value={ipDraft}
            onblur={commitIp}
            onkeydown={(e) => e.key === "Enter" && commitIp()}
            spellcheck="false"
          />
        </label>
        <label class="row">
          <span class="row-label">端口</span>
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
              <AlertCircle /> 端口必须是 1-65535 之间的整数
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
</style>
