<script lang="ts">
  import { Loader2, AlertCircle, Play, RefreshCw } from "lucide-svelte";
  import {
    ttsApplyDevice,
    ttsApplyVoice,
    ttsListDevices,
    ttsListVoices,
    ttsSetEnabled,
    ttsSpeak,
    ttsSwitchEngine,
    type Choice,
    type Engine,
  } from "../lib/ipc";
  import { store, pushToast } from "../lib/stores.svelte";
  import ModelDownload from "./ModelDownload.svelte";

  let devices = $state<Choice[]>([]);
  let voices = $state<Choice[]>([]);
  let previewing = $state(false);
  let refreshing = $state(false);

  // Re-fetch device/voice lists whenever the engine identity OR its
  // availability flips — covers both "user switched SAPI ↔ Sherpa" and
  // "model finished downloading so Sherpa just became usable". Skipping
  // the latter is the bug that made dropdowns stay empty after a fresh
  // model install until the user hit `刷新设备` by hand.
  let lastFetchedSig = "";
  $effect(() => {
    const e = store.tts.engine;
    if (e === "loading") return;
    const sig = `${e}:${store.tts.available}`;
    if (sig === lastFetchedSig) return;
    lastFetchedSig = sig;
    void refreshChoices();
  });

  async function refreshChoices() {
    refreshing = true;
    try {
      const [d, v] = await Promise.all([ttsListDevices(), ttsListVoices()]);
      devices = d;
      voices = v;
    } finally {
      // Visual lag for the spinning icon — too quick and the user can't
      // tell the click registered. Same cosmetic trick as the legacy app.
      setTimeout(() => (refreshing = false), 320);
    }
  }

  const cfg = $derived(store.config);
  const tts = $derived(store.tts);
  const isSherpa = $derived(cfg?.engine === "sherpa");

  async function pickEngine(engine: Engine) {
    if (cfg && cfg.engine === engine) return;
    await ttsSwitchEngine(engine);
    pushToast(engine === "sapi" ? "已切换到 Windows SAPI" : "已切换到 Sherpa AI");
  }

  async function pickDevice(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value;
    await ttsApplyDevice(value === "" ? null : value);
  }

  async function pickVoice(e: Event) {
    const value = (e.currentTarget as HTMLSelectElement).value;
    await ttsApplyVoice(value === "" ? null : value);
  }

  async function toggleEnabled() {
    if (!cfg) return;
    await ttsSetEnabled(!cfg.tts_enabled);
  }

  async function preview() {
    previewing = true;
    try {
      await ttsSpeak("VRChat 文本工具,语音预览。");
    } finally {
      setTimeout(() => (previewing = false), 1200);
    }
  }
</script>

<section class="section">
  <div class="section-label">语音播报</div>

  <div class="row toggle-row">
    <span>启用语音</span>
    <button
      class="toggle"
      class:on={cfg?.tts_enabled}
      type="button"
      onclick={toggleEnabled}
      aria-label="启用语音"
    >
      <span class="knob"></span>
    </button>
  </div>

  <div class="sub" class:disabled={!cfg?.tts_enabled}>
    <div class="row stacked">
      <span class="row-label">引擎</span>
      <div class="seg" role="radiogroup">
        <button
          type="button"
          class="seg-btn"
          class:on={cfg?.engine === "sapi"}
          onclick={() => pickEngine("sapi")}
        >
          Windows SAPI
        </button>
        <button
          type="button"
          class="seg-btn"
          class:on={cfg?.engine === "sherpa"}
          onclick={() => pickEngine("sherpa")}
        >
          Sherpa AI
        </button>
      </div>
    </div>

    {#if tts.loading}
      <div class="status info">
        <Loader2 class="spin" />
        AI 引擎加载中…
      </div>
    {:else if tts.error}
      <div class="status err">
        <AlertCircle />
        {tts.error}
      </div>
    {/if}

    {#if isSherpa}
      <ModelDownload />
    {/if}

    <label class="row stacked">
      <span class="row-label">声音</span>
      <div class="select-wrap">
        <select
          onchange={pickVoice}
          value={tts.current_voice ?? ""}
          disabled={voices.length === 0}
        >
          {#if voices.length === 0}
            <option value="">无可用声音</option>
          {:else}
            {#each voices as v (v.label)}
              <option value={v.key ?? ""}>{v.label}</option>
            {/each}
          {/if}
        </select>
      </div>
    </label>

    <label class="row stacked">
      <span class="row-label">输出设备</span>
      <div class="select-wrap">
        <select onchange={pickDevice} value={tts.current_device ?? ""}>
          {#each devices as d (d.label)}
            <option value={d.key ?? ""}>{d.label}</option>
          {/each}
        </select>
      </div>
    </label>

    <p class="hint">
      <span class="hint-mark">//</span>
      路由到 VRChat 麦克风需配合虚拟音频线缆(如 CABLE Input)作为输出设备
    </p>

    <div class="actions">
      <button
        class="preview"
        type="button"
        class:active={previewing}
        disabled={!tts.available || !cfg?.tts_enabled}
        onclick={preview}
      >
        <Play />
        {previewing ? "播放中…" : "试听"}
      </button>
      <button
        class="preview"
        type="button"
        class:active={refreshing}
        title="重新扫描音频设备和语音"
        onclick={refreshChoices}
      >
        <RefreshCw class={refreshing ? "spin" : ""} />
        刷新设备
      </button>
    </div>
  </div>
</section>

<style>
  .section {
    display: flex;
    flex-direction: column;
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
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
    font-size: 12px;
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: opacity 150ms;
  }
  .sub.disabled {
    opacity: 0.4;
    pointer-events: none;
  }

  /* Mini toggle pill */
  .toggle {
    width: 28px;
    height: 16px;
    border-radius: var(--r-pill);
    background: var(--card);
    position: relative;
    transition: background 150ms;
    flex-shrink: 0;
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: left 150ms, background 150ms;
  }
  .toggle.on .knob {
    left: 14px;
    background: var(--accent-text);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
  }
  .row-label {
    font-size: 11.5px;
    opacity: 0.7;
  }

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

  /* Select with custom chevron via background-image data URL — matches design.jsx */
  .select-wrap {
    position: relative;
  }
  select {
    width: 100%;
    appearance: none;
    background-color: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 5px 24px 5px 8px;
    outline: none;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='none' stroke='rgba(232,239,233,0.48)' stroke-width='1.4' d='M1 1l4 4 4-4'/></svg>");
    background-repeat: no-repeat;
    background-position: right 8px center;
    transition: border-color 120ms;
  }
  select:focus {
    border-color: var(--accent-ring);
  }
  select:disabled {
    color: var(--text-faint);
    cursor: not-allowed;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: var(--r-sm);
    font-size: 11.5px;
  }
  .status.info {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .status.err {
    background: color-mix(in oklab, var(--danger) 12%, transparent);
    color: var(--danger);
  }
  .status :global(svg) {
    width: 12px;
    height: 12px;
    stroke-width: 1.5;
  }
  .status :global(.spin) {
    animation: vrc-spin 700ms linear infinite;
  }

  /* `// terminal-comment` style hint — small, faint, with green slashes
   * matching `// HISTORY · N` in the main view. */
  .hint {
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--text-faint);
  }
  .hint-mark {
    color: var(--accent);
    opacity: 0.7;
    margin-right: 4px;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .preview {
    border: 1px solid var(--border);
    background: var(--card);
    color: var(--text);
    font-size: 11.5px;
    padding: 4px 10px;
    border-radius: var(--r-sm);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: background 120ms, color 120ms, border-color 120ms;
  }
  .preview:hover:not(:disabled) {
    border-color: var(--accent-ring);
    color: var(--accent);
  }
  .preview.active {
    background: var(--accent-soft);
    color: var(--accent);
    border-color: var(--accent-ring);
  }
  .preview:disabled {
    color: var(--text-faint);
    cursor: not-allowed;
  }
  .preview :global(svg) {
    width: 10px;
    height: 10px;
    stroke-width: 1.5;
  }
  .preview :global(.spin) {
    animation: vrc-spin 700ms linear infinite;
  }
</style>
