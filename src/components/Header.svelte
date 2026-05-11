<script lang="ts">
  import {
    Pin, PinOff, Bell, BellOff, Volume2, VolumeX, Settings as SettingsIcon,
  } from "lucide-svelte";
  import IconButton from "./IconButton.svelte";
  import { savePartialConfig, setAlwaysOnTop, ttsSetEnabled } from "../lib/ipc";
  import { store } from "../lib/stores.svelte";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    settingsOpen: boolean;
    onToggleSettings: () => void;
  }
  let { settingsOpen, onToggleSettings }: Props = $props();

  const cfg = $derived(store.config);

  async function togglePin() {
    if (!cfg) return;
    await setAlwaysOnTop(!cfg.always_on_top);
  }
  // Bell — VRChat's chatbox notification ding (the `sound` arg of /chatbox/input).
  // Pure config flag, no worker involved.
  async function toggleBell() {
    if (!cfg) return;
    await savePartialConfig({ play_sound: !cfg.play_sound });
  }
  // Volume — local text-to-speech readback. Goes through tts_set_enabled
  // (NOT save_partial_config) so the TTS worker also receives the flag and,
  // on enable, calls engine.reload() to re-acquire the audio device. This
  // is the legacy app's recovery path for a crashed `audiosrv` — toggle off
  // and back on to restore voice without restarting VRCText.
  async function toggleTts() {
    if (!cfg) return;
    await ttsSetEnabled(!cfg.tts_enabled);
  }
</script>

<header class="hdr">
  <div class="brand">
    <span
      class="dot"
      class:ok={store.oscStatus === "ok"}
      class:error={store.oscStatus === "error"}
    ></span>
    <span class="title">VRCText</span>
  </div>

  <div
    class="osc"
    class:ok={store.oscStatus === "ok"}
    class:error={store.oscStatus === "error"}
  >
    <span class="osc-label">OSC</span>
    <span class="osc-addr">{cfg ? `${cfg.ip}:${cfg.port}` : "—"}</span>
  </div>

  <div class="spacer"></div>

  <IconButton
    label={cfg?.always_on_top ? t("unpinWindow") : t("pinWindow")}
    active={!!cfg?.always_on_top}
    onclick={togglePin}
  >
    {#if cfg?.always_on_top}<Pin />{:else}<PinOff />{/if}
  </IconButton>
  <IconButton
    label={cfg?.play_sound ? t("disableBell") : t("enableBell")}
    active={!!cfg?.play_sound}
    onclick={toggleBell}
  >
    {#if cfg?.play_sound}<Bell />{:else}<BellOff />{/if}
  </IconButton>
  <IconButton
    label={cfg?.tts_enabled ? t("disableTts") : t("enableTts")}
    active={!!cfg?.tts_enabled}
    onclick={toggleTts}
  >
    {#if cfg?.tts_enabled}<Volume2 />{:else}<VolumeX />{/if}
  </IconButton>
  <IconButton
    label={settingsOpen ? t("closeSettings") : t("openSettings")}
    active={settingsOpen}
    onclick={onToggleSettings}
  >
    <SettingsIcon />
  </IconButton>
</header>

<style>
  .hdr {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px 6px;
    height: 44px;
    box-sizing: border-box;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  /* Square dot — terminal aesthetic.
   * Idle (no send yet) shows a flat faint grey, no glow.
   * "ok" (last send dispatched OK) glows accent green.
   * "error" (send failed at OS level) glows danger red. */
  .dot {
    width: 7px;
    height: 7px;
    background: var(--text-faint);
    transition: background 150ms, box-shadow 150ms;
  }
  .dot.ok {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .dot.error {
    background: var(--danger);
    box-shadow: 0 0 8px var(--danger);
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text);
  }
  /* `[OSC 127.0.0.1:9000]` style chip.
   * Same idle/ok/error tri-state as the dot — when nothing has been sent
   * yet, the label sits in muted grey; once a packet flies the chip glows
   * accent green; on send_to() failure it flips red. */
  .osc {
    margin-left: 4px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 7px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    transition: border-color 150ms, color 150ms;
  }
  .osc-label {
    color: var(--text-faint);
    transition: color 150ms;
  }
  .osc-addr {
    color: var(--text-faint);
    transition: color 150ms;
  }
  .osc.ok {
    border-color: var(--border-strong);
  }
  .osc.ok .osc-label {
    color: var(--accent);
  }
  .osc.ok .osc-addr {
    color: var(--text-muted);
  }
  .osc.error {
    border-color: var(--danger);
  }
  .osc.error .osc-label {
    color: var(--danger);
  }
  .osc.error .osc-addr {
    color: var(--text-muted);
  }
  .spacer {
    flex: 1;
  }
</style>
