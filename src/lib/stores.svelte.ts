// Svelte 5 runes-based stores. These are plain reactive objects exported
// from a module — components import and read/write `state.field` directly,
// which is reactive thanks to `$state`.
//
// Backend is always the source of truth; the frontend mirrors what
// `load_config` / `get_history` / `tts_status` returned and re-fetches on
// the matching events.

import {
  loadConfig,
  getHistory,
  ttsStatus,
  installedPacks,
  onConfigChanged,
  onHistoryChanged,
  onTtsStatus,
  onDownloadProgress,
  onDownloadComplete,
  onDownloadError,
  onVrcAlive,
  type Config,
  type HistoryEntry,
  type TtsStatus,
  type DownloadProgress,
  type ModelKindStr,
} from "./ipc";

export const MAX_CHARS = 144;

/** "ok" means we've heard VRChat broadcast OSC packets on port 9001 in the
 *  last few seconds (so OSC really is enabled on the other side); "idle"
 *  means no traffic, "error" means our own send_to() call failed at the
 *  OS layer. The status is driven by VRChat's outbound packets, not by
 *  whether we recently dispatched something — UDP send_to() always claims
 *  success regardless of whether anyone is listening. */
export type OscStatus = "idle" | "ok" | "error";

const STATUS_HOLD_MS = 3000;

let statusTimer: ReturnType<typeof setTimeout> | null = null;

/** Set the OSC status with a self-clearing timer. Each refresh re-arms
 *  the timer so a steady stream of `vrc-alive` events keeps the dot at
 *  "ok" indefinitely; when the stream stops we fade to idle. Errors take
 *  precedence over alive events for the duration of the hold so the user
 *  notices them. */
export function setOscStatus(s: OscStatus) {
  store.oscStatus = s;
  if (statusTimer !== null) clearTimeout(statusTimer);
  if (s === "idle") {
    statusTimer = null;
    return;
  }
  statusTimer = setTimeout(() => {
    store.oscStatus = "idle";
    statusTimer = null;
  }, STATUS_HOLD_MS);
}

interface AppStore {
  config: Config | null;
  history: HistoryEntry[];
  loaded: boolean;
  tts: TtsStatus;
  installedPacks: ModelKindStr[];
  download: DownloadProgress | null;
  /** Last-failed download, kept around so the UI can render a retry button
   *  tied to the specific pack that failed. Cleared when a new download
   *  starts or the user explicitly dismisses it. */
  downloadError: { kind: ModelKindStr; message: string } | null;
  oscStatus: OscStatus;
}

const defaultTts: TtsStatus = {
  engine: "sapi",
  available: false,
  loading: false,
  error: null,
  current_device: null,
  current_voice: null,
};

export const store: AppStore = $state({
  config: null,
  history: [],
  loaded: false,
  tts: defaultTts,
  installedPacks: [],
  download: null,
  downloadError: null,
  oscStatus: "idle",
});

/** Cross-component intent: HistoryRow short-clicks publish a "please paste
 *  this text into the composer at the cursor" request here; Composer's
 *  effect reads it, applies the splice, and writes back null. The `key`
 *  field forces Svelte to re-fire the effect even when the same text is
 *  appended twice in a row. */
export interface AppendIntent {
  text: string;
  key: number;
}
export const appendBus: { value: AppendIntent | null } = $state({ value: null });

export function requestAppend(text: string) {
  appendBus.value = { text, key: Date.now() + Math.random() };
}

/** Toast bus — same idea as `appendBus`. Components deep in the tree
 *  (like TtsSection inside Settings inside App) call `pushToast` rather
 *  than asking for an `onStatus` prop to be drilled through. App's effect
 *  consumes the bus and routes through the existing `setStatus` so all
 *  toasts share one render path. */
export interface ToastIntent {
  text: string;
  tone: "info" | "error";
  key: number;
}
export const toastBus: { value: ToastIntent | null } = $state({ value: null });

export function pushToast(text: string, tone: "info" | "error" = "info") {
  toastBus.value = { text, tone, key: Date.now() + Math.random() };
}

/** Wire the bridge listeners. Call once from the root component. */
export async function initStore(): Promise<() => void> {
  // Listeners are registered BEFORE the initial snapshot fetches. An event
  // that fires in between (e.g. the sherpa engine finishing its async load
  // during app init) would otherwise be lost, leaving the UI stuck on the
  // stale snapshot until the next unrelated event. The fetches are issued
  // after registration, so their results are never older than a missed
  // event.
  const unlistenConfig = await onConfigChanged((c) => {
    store.config = c;
  });
  const unlistenHistory = await onHistoryChanged(async () => {
    store.history = await getHistory();
  });
  const unlistenTts = await onTtsStatus((s) => {
    store.tts = s;
  });
  const unlistenDl = await onDownloadProgress((p) => {
    store.download = p;
    store.downloadError = null;
  });
  const unlistenDone = await onDownloadComplete(async () => {
    store.download = null;
    store.installedPacks = await installedPacks();
  });
  const unlistenErr = await onDownloadError((err) => {
    store.download = null;
    store.downloadError = { kind: err.kind, message: err.message };
  });

  // Track VRChat-alive presence — every received packet refreshes the
  // status timer. As long as VRChat keeps broadcasting (~60 Hz, throttled
  // to 1 event per second by the backend) the dot stays "ok". When the
  // stream stops, we fade to idle 3 s after the last packet.
  // An active "error" wins over an alive heartbeat for the hold duration
  // so failures don't get silently swallowed.
  const unlistenAlive = await onVrcAlive(() => {
    if (store.oscStatus === "error") return;
    setOscStatus("ok");
  });

  const [cfg, hist, tts, packs] = await Promise.all([
    loadConfig(),
    getHistory(),
    ttsStatus(),
    installedPacks(),
  ]);
  store.config = cfg;
  store.history = hist;
  store.tts = tts;
  store.installedPacks = packs;
  store.loaded = true;

  return () => {
    unlistenConfig();
    unlistenHistory();
    unlistenTts();
    unlistenDl();
    unlistenDone();
    unlistenErr();
    unlistenAlive();
    if (statusTimer !== null) clearTimeout(statusTimer);
  };
}
