// Typed wrappers around Tauri's invoke(). Keeping all bridge calls in one
// file means refactoring a command name only edits here; component code
// stays declarative.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ── Types ─────────────────────────────────────────────────────────────────

export type Engine = "sapi" | "sherpa";

export type Lang = "zh" | "en";

export interface HistoryEntry {
  text: string;
  ts: number;
}

export interface Config {
  ip: string;
  port: number;
  play_sound: boolean;
  always_on_top: boolean;
  tts_enabled: boolean;
  engine: Engine;
  tts_device_sapi: string | null;
  tts_voice_sapi: string | null;
  tts_device_sherpa: string | null;
  tts_voice_sherpa: string | null;
  language: Lang;
  history: HistoryEntry[];
}

export interface ConfigPatch {
  ip?: string;
  port?: number;
  play_sound?: boolean;
  always_on_top?: boolean;
  tts_enabled?: boolean;
  engine?: Engine;
  tts_device_sapi?: string | null;
  tts_voice_sapi?: string | null;
  tts_device_sherpa?: string | null;
  tts_voice_sherpa?: string | null;
  language?: Lang;
}

// ── Config / OSC / history ───────────────────────────────────────────────

export const loadConfig = () => invoke<Config>("load_config");

export const savePartialConfig = (patch: ConfigPatch) =>
  invoke<Config>("save_partial_config", { patch });

export const sendMessage = (text: string) =>
  invoke<void>("send_message", { text });

export const setTyping = (typing: boolean) =>
  invoke<void>("set_typing", { typing });

export const getHistory = () => invoke<HistoryEntry[]>("get_history");

export const clearHistory = () => invoke<void>("clear_history");

export const resendHistory = (index: number) =>
  invoke<void>("resend_history", { index });

export const setAlwaysOnTop = (pinned: boolean) =>
  invoke<void>("set_always_on_top", { pinned });

// ── TTS ──────────────────────────────────────────────────────────────────

export interface Choice {
  label: string;
  key: string | null;
}

export interface TtsStatus {
  engine: "sapi" | "sherpa" | "loading";
  available: boolean;
  loading: boolean;
  error: string | null;
  current_device: string | null;
  current_voice: string | null;
}

export const ttsStatus = () => invoke<TtsStatus>("tts_status");
export const ttsSpeak = (text: string) => invoke<void>("tts_speak", { text });
export const ttsStop = () => invoke<void>("tts_stop");
export const ttsSetEnabled = (enabled: boolean) =>
  invoke<void>("tts_set_enabled", { enabled });
export const ttsSwitchEngine = (engine: Engine) =>
  invoke<void>("tts_switch_engine", { engine });
export const ttsListDevices = () => invoke<Choice[]>("tts_list_devices");
export const ttsListVoices = () => invoke<Choice[]>("tts_list_voices");
export const ttsApplyDevice = (key: string | null) =>
  invoke<string | null>("tts_apply_device", { key });
export const ttsApplyVoice = (key: string | null) =>
  invoke<string | null>("tts_apply_voice", { key });

// ── Downloads ────────────────────────────────────────────────────────────

export type ModelKindStr = "matcha" | "kokoro";

export interface DownloadProgress {
  kind: ModelKindStr;
  status: string;
  progress: number;
}
export interface DownloadError {
  kind: ModelKindStr;
  message: string;
}

export const downloadPack = (kind: ModelKindStr) =>
  invoke<void>("download_pack", { kind });
export const deleteModels = () => invoke<void>("delete_models");
export const installedPacks = () => invoke<ModelKindStr[]>("installed_packs");

// ── Events ───────────────────────────────────────────────────────────────

export const onConfigChanged = (handler: (cfg: Config) => void): Promise<UnlistenFn> =>
  listen<Config>("config-changed", (e) => handler(e.payload));

export const onHistoryChanged = (handler: () => void): Promise<UnlistenFn> =>
  listen<unknown>("history-changed", () => handler());

export const onTtsStatus = (handler: (s: TtsStatus) => void): Promise<UnlistenFn> =>
  listen<TtsStatus>("tts-status", (e) => handler(e.payload));

export const onDownloadProgress = (
  handler: (p: DownloadProgress) => void,
): Promise<UnlistenFn> =>
  listen<DownloadProgress>("download-progress", (e) => handler(e.payload));

export const onDownloadComplete = (
  handler: (kind: ModelKindStr) => void,
): Promise<UnlistenFn> =>
  listen<ModelKindStr>("download-complete", (e) => handler(e.payload));

export const onDownloadError = (
  handler: (err: DownloadError) => void,
): Promise<UnlistenFn> =>
  listen<DownloadError>("download-error", (e) => handler(e.payload));

export const onVrcAlive = (handler: () => void): Promise<UnlistenFn> =>
  listen<unknown>("vrc-alive", () => handler());
