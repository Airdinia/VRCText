// Minimal i18n. The dictionary is a flat key→string table per language.
// `t(key)` reads `store.config.language` reactively because `store` is a
// `$state` object — Svelte tracks the access through `t`'s body and re-runs
// any template / `$derived` that called it when the language flips.

import { savePartialConfig, type Lang } from "./ipc";
import { store } from "./stores.svelte";

const dict = {
  zh: {
    // Header tooltips
    pinWindow: "窗口置顶",
    unpinWindow: "取消窗口置顶",
    enableBell: "开启 VRChat 提示音",
    disableBell: "关闭 VRChat 提示音",
    enableTts: "开启语音朗读",
    disableTts: "关闭语音朗读",
    openSettings: "打开设置",
    closeSettings: "关闭设置",

    // Composer
    composerPlaceholder: "$ 输入消息_",
    sendHint: "发送",
    sendButton: "发送",
    sendButtonTitle: "发送 (Enter)",
    sentToast: "已发送",
    overflowToast: "超过 {n} 字符上限",
    overflowCount: "超出 {n}",

    // Settings panel
    settings: "设置",
    close: "关闭",
    language: "语言",
    oscTarget: "OSC 目标",
    ipAddress: "IP 地址",
    port: "端口",
    portError: "端口必须是 1-65535 之间的整数",

    // TTS section
    ttsSection: "语音播报",
    enableVoice: "启用语音",
    engine: "引擎",
    voice: "声音",
    noVoice: "无可用声音",
    outputDevice: "输出设备",
    aiLoading: "AI 引擎加载中…",
    routingHint:
      "使用虚拟音频线缆将语音输出路由至 VRChat 的麦克风输入",
    preview: "试听",
    playing: "播放中…",
    refreshDevices: "刷新设备",
    refreshDevicesTitle: "重新扫描音频设备和语音",
    sapiEngine: "Windows 原生",
    sherpaEngine: "AI 模型",
    switchedToSapi: "已切换到 Windows 原生",
    switchedToSherpa: "已切换到 AI 模型",
    ttsPreviewSentence: "VRChat 文本工具,语音预览。",

    // History list
    historyHeader: "HISTORY",
    clear: "[CLEAR]",
    confirmClear: "[CONFIRM]",
    clearTitle: "清空历史",
    confirmClearTitle: "再点确认清空",
    emptyHistory: "还没有历史",
    emptyHint: "下面打字,Enter 发送",
    clearedToast: "已清空历史",

    // History row
    historyRowTooltip: "短按:追加到输入框｜长按 0.5s:直接重发",
    resentToast: "已重发 「{preview}」",
    resendFailedToast: "重发失败: {err}",

    // Model download
    matchaName: "Matcha 中文",
    kokoroName: "Kokoro 中英双语",
    installed: "已安装",
    download: "下载",
    retry: "重试",
    dismiss: "取消",
    deleteAllModels: "删除所有模型",
    confirmDeleteAll: "再点确认全部删除",
    downloadFailed: "下载失败: {message}",

    // Backend tokens (rendered via maybeT — see below)
    // TTS errors
    engineLoading: "AI 引擎加载中…",
    systemDefault: "系统默认",
    errModelsDir: "无法解析模型目录",
    errAppDataModels: "无法解析 %APPDATA%\\vrctext\\models",
    errNoModel: "未检测到 AI 模型，请先在下方下载",
    errLoadFailed: "模型文件存在但加载失败，可能是文件损坏",
    errAudioDefault: "打不开默认音频输出设备",
    errAudioDefaultHint: "打不开默认音频输出设备；请在设置里换一个输出设备",
    errAudioOpen: "打不开该音频设备",
    // Download status (static)
    dlPrepare: "准备下载 {arg}…",
    dlComplete: "完成",
    dlKokoro: "下载 Kokoro 模型 (~365 MB，全精度)",
    dlExtractLarge: "解压（文件较多，请耐心等待）…",
    // Download errors
    dlCreateDirFail: "创建目录失败",
    dlVerifyMissing: "解压完成但 {arg} 的关键文件缺失，请打开模型目录检查",
    dlHttp: "HTTP 请求失败",
    dlWriteFile: "写文件失败",
    dlInterrupted: "下载中断",
    dlWriteData: "写入失败",
    dlOpenArchive: "打开压缩包失败",
    dlExtractFail: "解压失败",
    dlAlreadyRunning: "已有下载在进行中",
    dlDeleteFail: "删除失败",
    // Other command errors
    errHistoryMissing: "历史项不存在",
    errOscTarget: "目标地址无效",
    errOscSend: "OSC 发送失败",
    errEmptyMessage: "空消息",
    dlSizeInvalid: "下载内容为空、大小不符或超出限制，请重试",
    dlChecksumMismatch: "模型校验失败，请重试；若仍失败，请报告问题",
    errInvalidMessage: "消息不得超过 144 个字符或包含空字符",
  },
  en: {
    // Header tooltips
    pinWindow: "Pin Window on Top",
    unpinWindow: "Unpin Window",
    enableBell: "Enable VRChat Notification Sound",
    disableBell: "Disable VRChat Notification Sound",
    enableTts: "Enable Voice Readout",
    disableTts: "Disable Voice Readout",
    openSettings: "Open Settings",
    closeSettings: "Close Settings",

    // Composer
    composerPlaceholder: "$ type a message_",
    sendHint: "Send",
    sendButton: "Send",
    sendButtonTitle: "Send (Enter)",
    sentToast: "Sent",
    overflowToast: "Exceeds the {n}-character limit",
    overflowCount: "+{n}",

    // Settings panel
    settings: "Settings",
    close: "Close",
    language: "Language",
    oscTarget: "OSC Target",
    ipAddress: "IP Address",
    port: "Port",
    portError: "Port must be an integer between 1 and 65535",

    // TTS section
    ttsSection: "Voice Readout",
    enableVoice: "Enable Voice",
    engine: "Engine",
    voice: "Voice",
    noVoice: "No Voice Available",
    outputDevice: "Output Device",
    aiLoading: "AI Engine Loading…",
    routingHint:
      "Use a virtual audio cable to route speech output to VRChat's microphone input",
    preview: "Preview",
    playing: "Playing…",
    refreshDevices: "Refresh",
    refreshDevicesTitle: "Re-scan audio devices and voices",
    sapiEngine: "Windows Native",
    sherpaEngine: "AI Model",
    switchedToSapi: "Switched to Windows Native",
    switchedToSherpa: "Switched to AI Model",
    ttsPreviewSentence: "VRChat text tool, voice preview.",

    // History list
    historyHeader: "HISTORY",
    clear: "[CLEAR]",
    confirmClear: "[CONFIRM]",
    clearTitle: "Clear History",
    confirmClearTitle: "Click Again to Confirm",
    emptyHistory: "No History Yet",
    emptyHint: "Type below, press Enter to send",
    clearedToast: "History Cleared",

    // History row
    historyRowTooltip:
      "Tap: append to composer | Hold 0.5s: resend directly",
    resentToast: "Resent 「{preview}」",
    resendFailedToast: "Resend failed: {err}",

    // Model download
    matchaName: "Matcha Chinese",
    kokoroName: "Kokoro Chinese / English",
    installed: "Installed",
    download: "Download",
    retry: "Retry",
    dismiss: "Dismiss",
    deleteAllModels: "Delete All Models",
    confirmDeleteAll: "Click Again to Confirm Delete",
    downloadFailed: "Download failed: {message}",

    // Backend tokens
    engineLoading: "AI Engine Loading…",
    systemDefault: "System Default",
    errModelsDir: "Cannot resolve models directory",
    errAppDataModels: "Cannot resolve %APPDATA%\\vrctext\\models",
    errNoModel: "No AI model detected — please download one below",
    errLoadFailed: "Model files exist but failed to load — they may be corrupted",
    errAudioDefault: "Cannot open the default audio output device",
    errAudioDefaultHint:
      "Cannot open the default audio output device; pick another in settings",
    errAudioOpen: "Cannot open this audio device",
    dlPrepare: "Preparing to download {arg}…",
    dlComplete: "Done",
    dlKokoro: "Downloading Kokoro model (~365 MB, full precision)",
    dlExtractLarge: "Extracting (this may take a while)…",
    dlCreateDirFail: "Failed to create directory",
    dlVerifyMissing:
      "Extraction finished but key files for {arg} are missing — please inspect the models directory",
    dlHttp: "HTTP request failed",
    dlWriteFile: "Failed to write file",
    dlInterrupted: "Download interrupted",
    dlWriteData: "Write failed",
    dlOpenArchive: "Failed to open archive",
    dlExtractFail: "Extraction failed",
    dlAlreadyRunning: "A download is already in progress",
    dlDeleteFail: "Delete failed",
    errHistoryMissing: "History item does not exist",
    errOscTarget: "Invalid target address",
    errOscSend: "OSC send failed",
    errEmptyMessage: "Empty message",
    dlSizeInvalid: "Download is empty, incomplete, or too large; please retry",
    dlChecksumMismatch: "Model checksum mismatch; retry or report the problem if it persists",
    errInvalidMessage: "Messages must be at most 144 characters and contain no NUL characters",
  },
} as const;

export type Key = keyof (typeof dict)["zh"];

function currentLang(): Lang {
  const l = store.config?.language;
  return l === "en" ? "en" : "zh";
}

/** Reactive string lookup. Falls back to zh if a key is missing in the
 *  selected language (defensive — should never happen since both tables
 *  share the same key set). */
export function t(key: Key): string {
  const lang = currentLang();
  return dict[lang][key] ?? dict.zh[key];
}

/** Interpolation helper for keys containing `{name}` placeholders. */
export function tf(key: Key, params: Record<string, string | number>): string {
  let s = t(key);
  for (const [k, v] of Object.entries(params)) {
    s = s.replaceAll(`{${k}}`, String(v));
  }
  return s;
}

/** Translate a backend-emitted string. The backend uses a stable token
 *  format so the resolved text can switch with UI language:
 *    "@i18n:keyName"            → look up `keyName`, return template
 *    "@i18n:keyName|payload"    → look up `keyName`. If the template
 *                                  contains `{arg}` we substitute payload
 *                                  in place; otherwise we append it after
 *                                  `: ` so error suffixes (HTTP errors,
 *                                  file paths, etc.) still surface.
 *  Payloads that themselves are dict keys (e.g. `matchaName`) get one
 *  level of recursive resolution so the embedded reference flips with
 *  the language too.
 *
 *  Strings that don't start with `@i18n:` are returned verbatim. */
const TOKEN_RE = /^@i18n:([A-Za-z][\w]*)(?:\|([\s\S]*))?$/;

export function maybeT(raw: string | null | undefined): string {
  if (!raw) return "";
  const m = TOKEN_RE.exec(raw);
  if (!m) return raw;
  const [, key, payload] = m;
  const lang = currentLang();
  const langDict = dict[lang] as Record<string, string>;
  const zhDict = dict.zh as Record<string, string>;
  const tmpl = langDict[key] ?? zhDict[key] ?? key;
  if (payload === undefined) return tmpl;
  // Resolve payload through the dict (one level) so `matchaName` /
  // `kokoroName` references swap with language.
  const resolved = langDict[payload] ?? zhDict[payload] ?? payload;
  return tmpl.includes("{arg}")
    ? tmpl.replace("{arg}", resolved)
    : `${tmpl}: ${resolved}`;
}

export async function setLanguage(lang: Lang): Promise<void> {
  await savePartialConfig({ language: lang });
}
