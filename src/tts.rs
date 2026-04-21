use std::collections::VecDeque;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use sherpa_onnx::{
    GenerationConfig, LinearResampler as KaldiResampler, OfflineTts, OfflineTtsConfig,
    OfflineTtsKokoroModelConfig, OfflineTtsMatchaModelConfig, OfflineTtsModelConfig,
};

/// Kokoro multi-lang v1.1 ships 103 speakers (3 English + 55 Chinese female
/// + 45 Chinese male). Full sid map:
///   - sid 0..=2   → af_maple, af_sol, bf_vale   (English)
///   - sid 3..=57  → zf_001 .. zf_055             (Chinese female, LongMaoData)
///   - sid 58..=102 → zm_058 .. zm_102             (Chinese male, LongMaoData)
///
/// The LongMao speakers have no descriptive names — they're anonymized
/// professional voice actors — so we label them by gender + ordinal, which
/// reads better in the picker than "zf_001". Advanced users can still set
/// any sid 0..=102 by hand-editing `tts_voice_sherpa` in config.toml.
const CURATED_KOKORO_SPEAKERS: &[(i32, &str)] = &[
    (3, "女声 · zf_001"),
];
use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Media::Audio::{waveOutGetDevCapsW, waveOutGetNumDevs, WAVEOUTCAPSW};
use windows::Win32::Media::Speech::{
    ISpMMSysAudio, ISpObjectToken, ISpObjectTokenCategory, ISpVoice, SpMMAudioOut,
    SpObjectTokenCategory, SpVoice, SPF_ASYNC, SPF_PURGEBEFORESPEAK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

const MMSYSERR_NOERROR: u32 = 0;
const DEVICE_DEFAULT: u32 = u32::MAX;
const DEFAULT_LABEL: &str = "系统默认";
const SPCAT_VOICES: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices";

/// A selectable item (audio device or voice) rendered in the settings UI.
/// `key = None` means "system default" — the sentinel is kept entirely inside
/// this module so callers never see `u32::MAX` or re-implement the default row.
#[derive(Clone, Debug)]
pub struct Choice {
    pub label: String,
    pub key: Option<String>,
}

/// Abstraction over a concrete TTS backend. Default is `SapiEngine` (Windows
/// SAPI, zero-dep); future backends like `SherpaEngine` (ONNX-based AI models)
/// implement the same surface so call sites never change.
pub trait TtsEngine {
    fn available(&self) -> bool;
    fn speak(&mut self, text: &str);
    fn stop(&mut self);
    fn device_choices(&self) -> Vec<Choice>;
    fn voice_choices(&self) -> Vec<Choice>;
    fn apply_device(&mut self, key: Option<&str>) -> Option<String>;
    fn apply_voice(&mut self, key: Option<&str>) -> Option<String>;
    /// Human-readable reason the engine is currently unavailable — only
    /// meaningful to call when `available()` is false. Default `None`
    /// keeps SAPI quiet; Sherpa uses it to tell the user whether the
    /// model failed to load vs the audio device rejected the stream.
    fn unavailable_detail(&self) -> Option<String> {
        None
    }
}

pub struct SapiEngine {
    voice: Option<ISpVoice>,
}

impl SapiEngine {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let voice = CoCreateInstance::<_, ISpVoice>(&SpVoice, None, CLSCTX_ALL).ok();
            Self { voice }
        }
    }
}

impl TtsEngine for SapiEngine {
    fn available(&self) -> bool {
        self.voice.is_some()
    }

    fn speak(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let Some(voice) = &self.voice else { return; };
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = SPF_ASYNC.0 as u32 | SPF_PURGEBEFORESPEAK.0 as u32;
        unsafe {
            let _ = voice.Speak(PCWSTR(wide.as_ptr()), flags, None);
        }
    }

    fn stop(&mut self) {
        let Some(voice) = &self.voice else { return; };
        let empty = [0u16];
        unsafe {
            let _ = voice.Speak(
                PCWSTR(empty.as_ptr()),
                SPF_PURGEBEFORESPEAK.0 as u32,
                None,
            );
        }
    }

    fn device_choices(&self) -> Vec<Choice> {
        let mut out = vec![Choice {
            label: DEFAULT_LABEL.into(),
            key: None,
        }];
        for (_, name) in enumerate_wave_out() {
            out.push(Choice {
                label: name.clone(),
                key: Some(name),
            });
        }
        out
    }

    fn voice_choices(&self) -> Vec<Choice> {
        let mut out = vec![Choice {
            label: DEFAULT_LABEL.into(),
            key: None,
        }];
        for (name, _) in enum_voice_tokens() {
            out.push(Choice {
                label: shorten_voice_label(&name),
                key: Some(name),
            });
        }
        out
    }

    fn apply_device(&mut self, key: Option<&str>) -> Option<String> {
        let Some(voice) = &self.voice else { return None; };
        if let Some(want) = key {
            if let Some((id, name)) = enumerate_wave_out()
                .into_iter()
                .find(|(_, name)| name == want)
            {
                if set_voice_output(voice, id) {
                    return Some(name);
                }
            }
        }
        let _ = unsafe { voice.SetOutput(None, true) };
        None
    }

    fn apply_voice(&mut self, key: Option<&str>) -> Option<String> {
        let Some(voice) = &self.voice else { return None; };
        unsafe {
            if let Some(want) = key {
                for (token_name, token) in enum_voice_tokens() {
                    if token_name == want && voice.SetVoice(&token).is_ok() {
                        return Some(token_name);
                    }
                }
            }
            let _ = voice.SetVoice(None);
            None
        }
    }
}

/// Transient engine used while the real `SherpaEngine` is being built on a
/// background thread. Keeps the app responsive during the ~1–2 s it takes
/// to load the ONNX acoustic model + vocoder; the UI treats it like any
/// other unavailable-with-detail engine and simply renders "加载中…".
pub struct LoadingEngine;

impl TtsEngine for LoadingEngine {
    fn available(&self) -> bool {
        false
    }
    fn speak(&mut self, _text: &str) {}
    fn stop(&mut self) {}
    fn device_choices(&self) -> Vec<Choice> {
        Vec::new()
    }
    fn voice_choices(&self) -> Vec<Choice> {
        Vec::new()
    }
    fn apply_device(&mut self, _key: Option<&str>) -> Option<String> {
        None
    }
    fn apply_voice(&mut self, _key: Option<&str>) -> Option<String> {
        None
    }
    fn unavailable_detail(&self) -> Option<String> {
        Some("AI 引擎加载中…".into())
    }
}

/// Pre-built Sherpa artifacts produced on a worker thread — the expensive
/// `OfflineTts::create` runs off the UI thread, then the main thread wraps
/// these into a full `SherpaEngine` with a cpal stream (which must be
/// created on the UI thread for WASAPI on Windows).
pub struct LoadedSherpa {
    pub tts: Arc<OfflineTts>,
    pub tts_sample_rate: u32,
    /// Identifies which pack + speaker was loaded, so `apply_voice` can
    /// short-circuit when the saved selection already matches.
    pub voice_key: Option<String>,
    pub sid: i32,
}

/// Spawn the Sherpa model load on a worker thread and return a channel the
/// UI thread can poll each frame. `preferred_key` is the saved voice
/// selection (`<pack>#<sid>` or just `<pack>`) — falls back to "first
/// discoverable pack + speaker 0" when `None` or unmatched.
pub fn load_sherpa_async(preferred_key: Option<String>) -> Receiver<Result<LoadedSherpa, String>> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let _ = tx.send(load_sherpa_sync(preferred_key.as_deref()));
    });
    rx
}

fn load_sherpa_sync(preferred_key: Option<&str>) -> Result<LoadedSherpa, String> {
    let dir = crate::config::models_dir().ok_or("无法解析模型目录")?;
    let (tts, resolved) = load_pack_by_key(&dir, preferred_key)
        .ok_or_else(|| "模型加载失败，可能是文件损坏".to_string())?;
    let rate = tts.sample_rate() as u32;
    let sid = resolved.as_ref().and_then(|k| parse_voice_key(k).1).unwrap_or(0);
    Ok(LoadedSherpa {
        tts: Arc::new(tts),
        tts_sample_rate: rate,
        voice_key: resolved,
        sid,
    })
}

/// sherpa-onnx-backed AI engine. Scans `%APPDATA%\vrctext\models\` for any
/// Matcha-TTS bundle and picks the first available; if no model is present,
/// `available()` reports false and the app's settings UI exposes a download
/// button. Audio routing uses cpal with user-selectable output device, and
/// synthesis runs on a background thread with streaming-callback playback.
pub struct SherpaEngine {
    tts: Option<Arc<OfflineTts>>,
    /// Native sample rate of the loaded model (Matcha zh-baker = 22050 Hz,
    /// Kokoro = 24000 Hz).
    tts_sample_rate: u32,
    /// Speaker index passed to `GenerationConfig.sid`. Always 0 for
    /// single-speaker packs like Matcha; Kokoro v1.0 uses 0..52.
    current_sid: i32,
    /// Currently loaded voice key (`<pack>` or `<pack>#<sid>`), reported to
    /// the UI so the combo stays in sync with reality.
    current_voice_key: Option<String>,
    /// Rate the audio device actually accepts (typically 48000 on Windows).
    /// Synthesized PCM is linear-resampled from `tts_sample_rate` to this.
    device_sample_rate: u32,
    /// Channel count the audio stream expects (usually 2). Mono synth output
    /// gets duplicated across channels at push time.
    device_channels: u16,
    /// Interleaved, device-ready PCM ready for the cpal callback to consume.
    buffer: Arc<Mutex<VecDeque<f32>>>,
    stream: Option<cpal::Stream>,
    current_device: Option<String>,
    /// Monotonic counter incremented on every `speak()` / `stop()` /
    /// `apply_voice()`. Each spawned synth captures the value at dispatch
    /// time; if the atomic advances during generation, its progress
    /// callback returns `false` and sherpa aborts the remaining steps —
    /// so interrupted runs never bleed samples into a newer utterance.
    gen_counter: Arc<AtomicU64>,
    /// Why this engine is currently unavailable, surfaced to the UI so the
    /// user gets a specific hint instead of a generic "未检测到 AI 模型"
    /// when the real problem is load failure or a rejected audio config.
    init_error: Option<String>,
}

struct DeviceStream {
    stream: cpal::Stream,
    device_name: Option<String>, // None = system default
    sample_rate: u32,
    channels: u16,
}

impl SherpaEngine {
    pub fn new() -> Self {
        let buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
        let (tts, tts_sample_rate, voice_key, dev, init_error) = Self::init(buffer.clone(), None);
        let (stream, device_sample_rate, device_channels) = match dev {
            Some(d) => (Some(d.stream), d.sample_rate, d.channels),
            None => (None, 48000, 2),
        };
        let current_sid = voice_key.as_deref().and_then(|k| parse_voice_key(k).1).unwrap_or(0);
        Self {
            tts,
            tts_sample_rate,
            current_sid,
            current_voice_key: voice_key,
            device_sample_rate,
            device_channels,
            buffer,
            stream,
            current_device: None,
            gen_counter: Arc::new(AtomicU64::new(0)),
            init_error,
        }
    }

    /// Attach a pre-loaded model (built on a worker thread via
    /// `load_sherpa_async`) and open the cpal stream on the current thread.
    /// Must be called on the UI thread — WASAPI streams are !Send and must
    /// be created where they're used.
    pub fn from_loaded(loaded: LoadedSherpa, device: Option<&str>) -> Self {
        let buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
        let dev = open_output_for(device, buffer.clone());
        let (stream, device_sample_rate, device_channels, init_error) = match dev {
            Some(d) => (Some(d.stream), d.sample_rate, d.channels, None),
            None => (None, 48000, 2, Some("打不开默认音频输出设备".into())),
        };
        Self {
            tts: Some(loaded.tts),
            tts_sample_rate: loaded.tts_sample_rate,
            current_sid: loaded.sid,
            current_voice_key: loaded.voice_key,
            device_sample_rate,
            device_channels,
            buffer,
            stream,
            current_device: None,
            gen_counter: Arc::new(AtomicU64::new(0)),
            init_error,
        }
    }

    /// Classify each step of the load so the UI can explain what went wrong.
    /// Returns (tts, tts_rate, voice_key, device_stream, error) where `error`
    /// is set when any step failed.
    fn init(
        buffer: Arc<Mutex<VecDeque<f32>>>,
        device: Option<&str>,
    ) -> (
        Option<Arc<OfflineTts>>,
        u32,
        Option<String>,
        Option<DeviceStream>,
        Option<String>,
    ) {
        let Some(models_dir) = crate::config::models_dir() else {
            return (
                None,
                22050,
                None,
                None,
                Some("无法解析 %APPDATA%\\vrctext\\models".into()),
            );
        };
        if !models_dir.exists()
            || std::fs::read_dir(&models_dir)
                .map(|mut i| i.next().is_none())
                .unwrap_or(true)
        {
            return (None, 22050, None, None, None); // no models yet — UI shows download
        }
        let (tts, voice_key) = match load_pack_by_key(&models_dir, None) {
            Some(x) => x,
            None => {
                return (
                    None,
                    22050,
                    None,
                    None,
                    Some("模型文件存在但加载失败，可能是文件损坏".into()),
                );
            }
        };
        let tts_sample_rate = tts.sample_rate() as u32;
        let dev = open_output_for(device, buffer);
        let err = if dev.is_none() {
            Some("打不开默认音频输出设备；请在设置里换一个输出设备".into())
        } else {
            None
        };
        (Some(Arc::new(tts)), tts_sample_rate, voice_key, dev, err)
    }
}

impl TtsEngine for SherpaEngine {
    fn available(&self) -> bool {
        self.tts.is_some() && self.stream.is_some()
    }

    fn unavailable_detail(&self) -> Option<String> {
        self.init_error.clone()
    }

    fn speak(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let Some(tts) = self.tts.clone() else {
            return;
        };
        // Purge-before-speak parity with SAPI: any queued audio stops now,
        // and any in-flight synth observes a bumped counter and aborts.
        let my_id = self.gen_counter.fetch_add(1, Ordering::SeqCst) + 1;
        self.buffer.lock().unwrap().clear();

        let buffer = self.buffer.clone();
        let counter = self.gen_counter.clone();
        let in_rate = self.tts_sample_rate;
        let out_rate = self.device_sample_rate;
        let channels = self.device_channels as usize;
        let sid = self.current_sid;
        let text = text.to_string();
        thread::spawn(move || {
            // Kaldi-style polyphase resampler with 6-tap sinc + anti-alias
            // LP at 0.99·Nyquist. A naive linear interp here leaves audible
            // high-frequency aliases on Kokoro (24 kHz source has clean
            // sibilants; mirror images fold into the 12–18 kHz band and
            // sound like electronic tones). sherpa ships the real thing.
            let resampler = match KaldiResampler::create(in_rate as i32, out_rate as i32) {
                Some(r) => Arc::new(r),
                None => return,
            };
            let buffer_cb = buffer.clone();
            let counter_cb = counter.clone();
            let resampler_cb = resampler.clone();
            // sherpa fires this callback multiple times during synth so
            // playback starts well before the full utterance is rendered.
            let callback = move |samples: &[f32], _progress: f32| -> bool {
                if counter_cb.load(Ordering::SeqCst) != my_id {
                    return false;
                }
                let resampled = resampler_cb.resample(samples, false);
                let mut buf = buffer_cb.lock().unwrap();
                buf.reserve(resampled.len() * channels);
                for s in resampled {
                    // Mono → N-channel: duplicate across every slot.
                    for _ in 0..channels {
                        buf.push_back(s);
                    }
                }
                true
            };
            let gen = GenerationConfig { sid, ..Default::default() };
            let _ = tts.generate_with_config(&text, &gen, Some(callback));
            // Flush the resampler's filter delay line so trailing samples
            // (~6 input samples ≈ 0.25 ms at 24 kHz) don't get chopped.
            if counter.load(Ordering::SeqCst) == my_id {
                let tail = resampler.resample(&[], true);
                if !tail.is_empty() {
                    let mut buf = buffer.lock().unwrap();
                    buf.reserve(tail.len() * channels);
                    for s in tail {
                        for _ in 0..channels {
                            buf.push_back(s);
                        }
                    }
                }
            }
        });
    }

    fn stop(&mut self) {
        // Cancel any running synth *and* drop already-queued audio so the
        // user hears silence immediately.
        self.gen_counter.fetch_add(1, Ordering::SeqCst);
        self.buffer.lock().unwrap().clear();
    }

    fn device_choices(&self) -> Vec<Choice> {
        let mut out = vec![Choice {
            label: DEFAULT_LABEL.into(),
            key: None,
        }];
        if let Ok(devices) = cpal::default_host().output_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    out.push(Choice {
                        label: name.clone(),
                        key: Some(name),
                    });
                }
            }
        }
        out
    }

    fn voice_choices(&self) -> Vec<Choice> {
        // Unlike devices (where the OS default follows user preference
        // over time), an AI voice is concrete — "系统默认" would just be a
        // relabel of whichever speaker happened to scan first, which users
        // rightly find confusing. So we list actual voices only.
        let mut out = Vec::new();
        let Some(models_dir) = crate::config::models_dir() else {
            return out;
        };
        for pack in discover_voice_packs(&models_dir) {
            match pack.kind {
                PackKind::Matcha => out.push(Choice {
                    label: format!("Matcha · {}", pack.name),
                    key: Some(pack.name),
                }),
                PackKind::Kokoro => {
                    // Kokoro v1.0 ships 53 speakers but only the Chinese-
                    // native ones sound natural on zh-with-English text, so
                    // we expose a curated subset. Users who want another
                    // sid can still set it by editing `tts_voice_sherpa` in
                    // config.toml — the loader parses any `#N`.
                    for (sid, name) in CURATED_KOKORO_SPEAKERS {
                        out.push(Choice {
                            label: format!("Kokoro · {name}"),
                            key: Some(format!("{}#{sid}", pack.name)),
                        });
                    }
                }
            }
        }
        out
    }

    fn apply_device(&mut self, key: Option<&str>) -> Option<String> {
        if self.tts.is_none() {
            return None;
        }
        // Drop the old stream first — cpal will re-acquire the device.
        self.stream = None;
        self.buffer.lock().unwrap().clear();
        let dev = open_output_for(key, self.buffer.clone());
        match dev {
            Some(d) => {
                let resolved = d.device_name.clone();
                self.device_sample_rate = d.sample_rate;
                self.device_channels = d.channels;
                self.stream = Some(d.stream);
                self.current_device = resolved.clone();
                self.init_error = None;
                resolved
            }
            None => {
                self.init_error = Some("打不开该音频设备".into());
                None
            }
        }
    }

    fn apply_voice(&mut self, key: Option<&str>) -> Option<String> {
        let Some(models_dir) = crate::config::models_dir() else {
            return None;
        };
        // Cancel pending synths on the old voice before we swap it out,
        // otherwise leftover callbacks would push samples from the previous
        // model into the new voice's output stream.
        self.gen_counter.fetch_add(1, Ordering::SeqCst);
        self.buffer.lock().unwrap().clear();

        // Fast path: Kokoro changes that only flip speaker ID don't need an
        // ONNX reload — the model itself stays in memory, we just retarget
        // `GenerationConfig.sid` on the next `speak()`. Saves several
        // seconds when scrolling through the combo.
        if let (Some(want), Some(cur)) = (key, self.current_voice_key.as_deref()) {
            let (new_pack, new_sid) = parse_voice_key(want);
            let (cur_pack, _) = parse_voice_key(cur);
            if new_pack == cur_pack && self.tts.is_some() {
                self.current_sid = new_sid.unwrap_or(0);
                self.current_voice_key = Some(want.to_string());
                return Some(want.to_string());
            }
        }

        self.stream = None;
        self.tts = None;
        let (new_tts, resolved_key) = match load_pack_by_key(&models_dir, key) {
            Some((tts, k)) => (Some(tts), k),
            None => (None, None),
        };
        let Some(tts) = new_tts else {
            return None;
        };
        self.tts_sample_rate = tts.sample_rate() as u32;
        self.tts = Some(Arc::new(tts));
        self.current_sid = resolved_key
            .as_deref()
            .and_then(|k| parse_voice_key(k).1)
            .unwrap_or(0);
        self.current_voice_key = resolved_key.clone();
        let dev = open_output_for(self.current_device.as_deref(), self.buffer.clone());
        if let Some(d) = dev {
            self.device_sample_rate = d.sample_rate;
            self.device_channels = d.channels;
            self.stream = Some(d.stream);
        }
        resolved_key
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackKind {
    Matcha,
    Kokoro,
}

#[derive(Clone, Debug)]
struct VoicePack {
    kind: PackKind,
    name: String,
}

/// Provider string for every `OfflineTtsConfig` we build — `VRCTEXT_SHERPA_PROVIDER=cuda`
/// switches to GPU inference when the exe links a CUDA-enabled sherpa-onnx.
fn provider() -> Option<String> {
    std::env::var("VRCTEXT_SHERPA_PROVIDER")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Voice keys are `"<pack>"` or `"<pack>#<sid>"`. Returns `(pack, sid)` where
/// `sid` is `None` for the bare form (Matcha single-speaker).
fn parse_voice_key(key: &str) -> (&str, Option<i32>) {
    match key.rsplit_once('#') {
        Some((pack, sid)) => (pack, sid.parse().ok()),
        None => (key, None),
    }
}

/// Walk `models_dir` and classify every subdirectory that looks like a known
/// voice pack. The shared `vocos-22khz-univ.onnx` vocoder (used by Matcha)
/// lives at the parent level, so it's checked there, not inside the pack dir.
fn discover_voice_packs(models_dir: &Path) -> Vec<VoicePack> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(models_dir) else {
        return out;
    };
    let vocos_present = models_dir.join("vocos-22khz-univ.onnx").exists();
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue; };
        if is_matcha_pack(&path) && vocos_present {
            out.push(VoicePack { kind: PackKind::Matcha, name: name.to_string() });
        } else if is_kokoro_pack(&path) {
            out.push(VoicePack { kind: PackKind::Kokoro, name: name.to_string() });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn is_matcha_pack(dir: &Path) -> bool {
    matcha_acoustic(dir).is_some() && dir.join("tokens.txt").exists()
}

fn is_kokoro_pack(dir: &Path) -> bool {
    // Accept both the full `model.onnx` and the int8-quantized `model.int8.onnx`
    // shipped by the v1.1 release — sherpa's C API picks whichever the config
    // points at (see `try_load_kokoro`).
    let model = dir.join("model.onnx").exists() || dir.join("model.int8.onnx").exists();
    model
        && dir.join("voices.bin").exists()
        && dir.join("tokens.txt").exists()
        && dir.join("espeak-ng-data").is_dir()
}

/// Matcha releases publish either `model-steps-3.onnx` or `model-steps-6.onnx`
/// depending on speaker count. Return whichever variant exists.
fn matcha_acoustic(voice_dir: &Path) -> Option<PathBuf> {
    for name in ["model-steps-3.onnx", "model-steps-6.onnx"] {
        let p = voice_dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn try_load_matcha(models_dir: &Path, pack_name: &str) -> Option<OfflineTts> {
    let voice_dir = models_dir.join(pack_name);
    let acoustic = matcha_acoustic(&voice_dir)?;
    let vocoder = models_dir.join("vocos-22khz-univ.onnx");
    let tokens = voice_dir.join("tokens.txt");
    if !vocoder.exists() || !tokens.exists() {
        return None;
    }
    let lexicon = voice_dir.join("lexicon.txt");
    let dict = voice_dir.join("dict");
    let matcha = OfflineTtsMatchaModelConfig {
        acoustic_model: Some(acoustic.to_string_lossy().into_owned()),
        vocoder: Some(vocoder.to_string_lossy().into_owned()),
        lexicon: lexicon.exists().then(|| lexicon.to_string_lossy().into_owned()),
        tokens: Some(tokens.to_string_lossy().into_owned()),
        dict_dir: dict.is_dir().then(|| dict.to_string_lossy().into_owned()),
        ..Default::default()
    };
    // Matcha zh-baker ships phone/date/number FST rules alongside the voice;
    // without wiring them up, `OfflineTts::create` refuses to initialise on
    // Chinese bundles. Pass whichever subset is present, in the canonical
    // order used by the upstream run-matcha-tts-zh.sh script.
    let mut fsts = Vec::new();
    for name in ["phone.fst", "date.fst", "number.fst"] {
        let p = voice_dir.join(name);
        if p.exists() {
            fsts.push(p.to_string_lossy().into_owned());
        }
    }
    let rule_fsts = if fsts.is_empty() { None } else { Some(fsts.join(",")) };
    let config = OfflineTtsConfig {
        model: OfflineTtsModelConfig {
            matcha,
            num_threads: 2,
            provider: provider(),
            ..Default::default()
        },
        rule_fsts,
        ..Default::default()
    };
    OfflineTts::create(&config)
}

fn try_load_kokoro(models_dir: &Path, pack_name: &str) -> Option<OfflineTts> {
    let d = models_dir.join(pack_name);
    if !is_kokoro_pack(&d) {
        return None;
    }
    // Assemble whichever lexicons shipped. The multi-lang pack ships
    // US/GB English plus Chinese; sherpa takes them as a comma-separated
    // list, and falls back to the espeak phonemizer when a language's
    // lexicon is missing.
    let mut lexicons: Vec<String> = Vec::new();
    for name in ["lexicon-us-en.txt", "lexicon-gb-en.txt", "lexicon-zh.txt"] {
        let p = d.join(name);
        if p.exists() {
            lexicons.push(p.to_string_lossy().into_owned());
        }
    }
    let lexicon = if lexicons.is_empty() { None } else { Some(lexicons.join(",")) };

    let dict = d.join("dict");
    let espeak = d.join("espeak-ng-data");

    // Multi-lang Kokoro takes Chinese FSTs too; en-only packs don't ship
    // them, so pass only what's present.
    let mut fsts = Vec::new();
    for name in ["phone-zh.fst", "date-zh.fst", "number-zh.fst"] {
        let p = d.join(name);
        if p.exists() {
            fsts.push(p.to_string_lossy().into_owned());
        }
    }
    let rule_fsts = if fsts.is_empty() { None } else { Some(fsts.join(",")) };

    // Prefer the quantized weights when they're present — they're half the
    // size and RTF and the quality delta is inaudible for chatbox TTS.
    let model_file = {
        let int8 = d.join("model.int8.onnx");
        if int8.exists() { int8 } else { d.join("model.onnx") }
    };
    let kokoro = OfflineTtsKokoroModelConfig {
        model: Some(model_file.to_string_lossy().into_owned()),
        voices: Some(d.join("voices.bin").to_string_lossy().into_owned()),
        tokens: Some(d.join("tokens.txt").to_string_lossy().into_owned()),
        data_dir: Some(espeak.to_string_lossy().into_owned()),
        dict_dir: dict.is_dir().then(|| dict.to_string_lossy().into_owned()),
        lexicon,
        length_scale: 1.0,
        ..Default::default()
    };
    let config = OfflineTtsConfig {
        model: OfflineTtsModelConfig {
            kokoro,
            num_threads: 2,
            provider: provider(),
            ..Default::default()
        },
        rule_fsts,
        ..Default::default()
    };
    OfflineTts::create(&config)
}

fn load_pack(models_dir: &Path, pack: &VoicePack) -> Option<OfflineTts> {
    match pack.kind {
        PackKind::Matcha => try_load_matcha(models_dir, &pack.name),
        PackKind::Kokoro => try_load_kokoro(models_dir, &pack.name),
    }
}

/// Try to load the pack that `preferred_key` refers to; fall back to the
/// first discoverable pack if the key is `None` or unresolvable. Returns
/// (engine, resolved_key) so the caller can persist the resolved form.
fn load_pack_by_key(
    models_dir: &Path,
    preferred_key: Option<&str>,
) -> Option<(OfflineTts, Option<String>)> {
    let packs = discover_voice_packs(models_dir);
    if let Some(key) = preferred_key {
        let (want_pack, want_sid) = parse_voice_key(key);
        if let Some(pack) = packs.iter().find(|p| p.name == want_pack) {
            if let Some(tts) = load_pack(models_dir, pack) {
                let resolved = match pack.kind {
                    PackKind::Matcha => pack.name.clone(),
                    PackKind::Kokoro => format!("{}#{}", pack.name, want_sid.unwrap_or(0)),
                };
                return Some((tts, Some(resolved)));
            }
        }
    }
    // Fallback: first pack that successfully loads.
    for pack in &packs {
        if let Some(tts) = load_pack(models_dir, pack) {
            let resolved = match pack.kind {
                PackKind::Matcha => pack.name.clone(),
                PackKind::Kokoro => format!("{}#0", pack.name),
            };
            return Some((tts, Some(resolved)));
        }
    }
    None
}

/// Open a cpal output stream on the named device (or system default when
/// `preferred` is `None`/unmatched). Uses the device's native sample rate
/// and channel count so WASAPI doesn't reject us — resampling to that rate
/// is the caller's responsibility (see `speak()` / `LinearResampler`).
fn open_output_for(
    preferred: Option<&str>,
    buffer: Arc<Mutex<VecDeque<f32>>>,
) -> Option<DeviceStream> {
    let host = cpal::default_host();
    let (device, used_name) = if let Some(want) = preferred {
        let picked = host
            .output_devices()
            .ok()
            .and_then(|mut it| it.find(|d| d.name().ok().as_deref() == Some(want)));
        match picked {
            Some(d) => (d, Some(want.to_string())),
            None => (host.default_output_device()?, None),
        }
    } else {
        (host.default_output_device()?, None)
    };
    let supported = device.default_output_config().ok()?;
    let config: cpal::StreamConfig = supported.config();
    let stream = build_stream_on(&device, &config, buffer)?;
    Some(DeviceStream {
        stream,
        device_name: used_name,
        sample_rate: config.sample_rate.0,
        channels: config.channels,
    })
}

fn build_stream_on(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<VecDeque<f32>>>,
) -> Option<cpal::Stream> {
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [f32], _| {
                let mut buf = buffer.lock().unwrap();
                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);
                }
            },
            |_err| {
                // Errors surface as audio gaps; no console in release builds.
            },
            None,
        )
        .ok()?;
    stream.play().ok()?;
    Some(stream)
}

fn set_voice_output(voice: &ISpVoice, device_id: u32) -> bool {
    unsafe {
        if device_id == DEVICE_DEFAULT {
            return voice.SetOutput(None, true).is_ok();
        }
        let audio: ISpMMSysAudio = match CoCreateInstance(&SpMMAudioOut, None, CLSCTX_ALL) {
            Ok(a) => a,
            Err(_) => return false,
        };
        if audio.SetDeviceId(device_id).is_err() {
            return false;
        }
        let unk: windows::core::IUnknown = match audio.cast() {
            Ok(u) => u,
            Err(_) => return false,
        };
        voice.SetOutput(&unk, true).is_ok()
    }
}

/// Trim SAPI voice descriptions for compact display in the UI.
/// Example: "Microsoft Huihui Desktop - Chinese (Simplified, PRC)"
///       → "Microsoft Huihui"
fn shorten_voice_label(name: &str) -> String {
    let base = name.split(" - ").next().unwrap_or(name);
    base.replace(" Desktop", "").replace(" Server", "").trim().to_string()
}

fn enumerate_wave_out() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let n = waveOutGetNumDevs();
        for i in 0..n {
            let mut caps: WAVEOUTCAPSW = std::mem::zeroed();
            let res = waveOutGetDevCapsW(
                i as usize,
                &mut caps,
                std::mem::size_of::<WAVEOUTCAPSW>() as u32,
            );
            if res != MMSYSERR_NOERROR {
                continue;
            }
            let pname: [u16; 32] = std::ptr::addr_of!(caps.szPname).read_unaligned();
            let len = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
            let name = OsString::from_wide(&pname[..len])
                .to_string_lossy()
                .into_owned();
            if !name.is_empty() {
                out.push((i, name));
            }
        }
    }
    out
}

fn enum_voice_tokens() -> Vec<(String, ISpObjectToken)> {
    let mut out = Vec::new();
    unsafe {
        let cat: ISpObjectTokenCategory =
            match CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL) {
                Ok(c) => c,
                Err(_) => return out,
            };
        let cid: Vec<u16> = SPCAT_VOICES
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        if cat.SetId(PCWSTR(cid.as_ptr()), false).is_err() {
            return out;
        }
        let enum_tokens = match cat.EnumTokens(PCWSTR::null(), PCWSTR::null()) {
            Ok(e) => e,
            Err(_) => return out,
        };
        loop {
            let mut slot: Option<ISpObjectToken> = None;
            let mut fetched: u32 = 0;
            if enum_tokens
                .Next(1, &mut slot as *mut _, Some(&mut fetched as *mut _))
                .is_err()
                || fetched == 0
            {
                break;
            }
            let Some(token) = slot.take() else { continue; };
            let desc = match token.GetStringValue(PCWSTR::null()) {
                Ok(p) => pwstr_to_string_and_free(p),
                Err(_) => continue,
            };
            if !desc.is_empty() {
                out.push((desc, token));
            }
        }
    }
    out
}

unsafe fn pwstr_to_string_and_free(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *p.0.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(p.0, len);
    let s = OsString::from_wide(slice).to_string_lossy().into_owned();
    CoTaskMemFree(Some(p.0 as _));
    s
}
