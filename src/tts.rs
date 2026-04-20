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
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsMatchaModelConfig,
    OfflineTtsModelConfig,
};
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
}

/// Spawn the Sherpa model load on a worker thread and return a channel the
/// UI thread can poll each frame.
pub fn load_sherpa_async() -> Receiver<Result<LoadedSherpa, String>> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let _ = tx.send(load_sherpa_sync());
    });
    rx
}

fn load_sherpa_sync() -> Result<LoadedSherpa, String> {
    let dir = crate::config::models_dir().ok_or("无法解析模型目录")?;
    let (tts, _) = load_matcha_auto(&dir);
    let tts = tts.ok_or_else(|| "模型加载失败，可能是文件损坏".to_string())?;
    let rate = tts.sample_rate() as u32;
    Ok(LoadedSherpa {
        tts: Arc::new(tts),
        tts_sample_rate: rate,
    })
}

/// sherpa-onnx-backed AI engine. Scans `%APPDATA%\vrctext\models\` for any
/// Matcha-TTS bundle and picks the first available; if no model is present,
/// `available()` reports false and the app's settings UI exposes a download
/// button. Audio routing uses cpal with user-selectable output device, and
/// synthesis runs on a background thread with streaming-callback playback.
pub struct SherpaEngine {
    tts: Option<Arc<OfflineTts>>,
    /// Native sample rate of the loaded model (Matcha zh-baker = 22050 Hz).
    tts_sample_rate: u32,
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
        let (tts, tts_sample_rate, dev, init_error) = Self::init(buffer.clone(), None);
        let (stream, device_sample_rate, device_channels) = match dev {
            Some(d) => (Some(d.stream), d.sample_rate, d.channels),
            None => (None, 48000, 2),
        };
        Self {
            tts,
            tts_sample_rate,
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
    /// Returns (tts, tts_rate, device_stream, error) where `error` is set
    /// when any step failed.
    fn init(
        buffer: Arc<Mutex<VecDeque<f32>>>,
        device: Option<&str>,
    ) -> (Option<Arc<OfflineTts>>, u32, Option<DeviceStream>, Option<String>) {
        let Some(models_dir) = crate::config::models_dir() else {
            return (
                None,
                22050,
                None,
                Some("无法解析 %APPDATA%\\vrctext\\models".into()),
            );
        };
        if !models_dir.exists()
            || std::fs::read_dir(&models_dir)
                .map(|mut i| i.next().is_none())
                .unwrap_or(true)
        {
            return (None, 22050, None, None); // no models yet — UI shows download
        }
        let (tts, _) = load_matcha_auto(&models_dir);
        let Some(tts) = tts else {
            return (
                None,
                22050,
                None,
                Some("模型文件存在但加载失败，可能是文件损坏".into()),
            );
        };
        let tts_sample_rate = tts.sample_rate() as u32;
        let dev = open_output_for(device, buffer);
        let err = if dev.is_none() {
            Some("打不开默认音频输出设备；请在设置里换一个输出设备".into())
        } else {
            None
        };
        (Some(Arc::new(tts)), tts_sample_rate, dev, err)
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
        let text = text.to_string();
        thread::spawn(move || {
            let mut resampler = LinearResampler::new(in_rate, out_rate);
            // sherpa fires this callback multiple times during synth, so
            // playback starts well before the full utterance is rendered.
            let callback = move |samples: &[f32], _progress: f32| -> bool {
                if counter.load(Ordering::SeqCst) != my_id {
                    return false;
                }
                let resampled = resampler.process(samples);
                let mut buf = buffer.lock().unwrap();
                buf.reserve(resampled.len() * channels);
                for s in resampled {
                    // Mono → N-channel: duplicate the sample across every slot.
                    for _ in 0..channels {
                        buf.push_back(s);
                    }
                }
                true
            };
            let _ = tts.generate_with_config(&text, &GenerationConfig::default(), Some(callback));
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
        // relabel of whichever voice happened to scan first, which users
        // rightly find confusing. So we list actual voices only.
        let mut out = Vec::new();
        let Some(models_dir) = crate::config::models_dir() else {
            return out;
        };
        for name in discover_matcha_voices(&models_dir) {
            out.push(Choice {
                label: name.clone(),
                key: Some(name),
            });
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
        self.stream = None;
        self.tts = None;
        let (new_tts, name) = match key {
            Some(want) => match try_load_matcha_named(&models_dir, want) {
                Some(tts) => (Some(tts), Some(want.to_string())),
                // Mirror SAPI: unmatched key falls back to the default pick.
                None => load_matcha_auto(&models_dir),
            },
            None => load_matcha_auto(&models_dir),
        };
        let Some(tts) = new_tts else {
            return None;
        };
        self.tts_sample_rate = tts.sample_rate() as u32;
        self.tts = Some(Arc::new(tts));
        let dev = open_output_for(self.current_device.as_deref(), self.buffer.clone());
        if let Some(d) = dev {
            self.device_sample_rate = d.sample_rate;
            self.device_channels = d.channels;
            self.stream = Some(d.stream);
        }
        name
    }
}

/// Walk `models_dir` and list every subdirectory that looks like a Matcha
/// bundle (contains an acoustic model + tokens). The `vocos` vocoder is
/// shared across voices, so it's checked against the parent dir, not here.
fn discover_matcha_voices(models_dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(models_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        if matcha_acoustic(&path).is_some() && path.join("tokens.txt").exists() {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
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

fn try_load_matcha_named(models_dir: &Path, voice_name: &str) -> Option<OfflineTts> {
    let voice_dir = models_dir.join(voice_name);
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
    let rule_fsts = if fsts.is_empty() {
        None
    } else {
        Some(fsts.join(","))
    };
    let config = OfflineTtsConfig {
        model: OfflineTtsModelConfig {
            matcha,
            num_threads: 2,
            // `VRCTEXT_SHERPA_PROVIDER=cuda` switches to GPU inference, but
            // only works when the exe is built against a CUDA-enabled
            // sherpa-onnx library (via `SHERPA_ONNX_LIB_DIR` at build time).
            // Default CPU is plenty for VRChat-sized sentences (<144 chars,
            // realtime factor ~0.1 on a modern laptop CPU).
            provider: std::env::var("VRCTEXT_SHERPA_PROVIDER")
                .ok()
                .filter(|s| !s.is_empty()),
            ..Default::default()
        },
        rule_fsts,
        ..Default::default()
    };
    OfflineTts::create(&config)
}

/// When no voice preference is set, pick the first voice the scan finds.
/// Returns the engine plus the resolved voice name (for persistence).
fn load_matcha_auto(models_dir: &Path) -> (Option<OfflineTts>, Option<String>) {
    for name in discover_matcha_voices(models_dir) {
        if let Some(tts) = try_load_matcha_named(models_dir, &name) {
            return (Some(tts), Some(name));
        }
    }
    (None, None)
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

/// Linear-interpolation resampler with inter-chunk state. Good enough for
/// speech TTS going 22050 → 48000 Hz; a full polyphase FIR would win on
/// noise floor but costs code and compile time we don't want to spend.
struct LinearResampler {
    /// Input samples consumed per output sample emitted (e.g. 22050/48000).
    step: f64,
    /// Fractional position into the current chunk (input-sample units).
    /// Negative values mean "the next output sits between the previous
    /// chunk's last sample and this chunk's first sample".
    pos: f64,
    /// Last input sample of the previous chunk — holds interpolation context
    /// across chunk boundaries so we don't discontinuity the audio.
    prev: f32,
}

impl LinearResampler {
    fn new(in_rate: u32, out_rate: u32) -> Self {
        Self {
            step: in_rate as f64 / out_rate as f64,
            pos: 0.0,
            prev: 0.0,
        }
    }

    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let n = input.len();
        if n == 0 {
            return Vec::new();
        }
        let approx_out = ((n as f64 / self.step).ceil() as usize).saturating_add(1);
        let mut out = Vec::with_capacity(approx_out);
        // Emit every output sample whose base index + 1 is still inside the
        // current chunk, so we always have both neighbours for interpolation.
        while self.pos.floor() as isize + 1 < n as isize {
            let base_f = self.pos.floor();
            let base = base_f as isize;
            let frac = (self.pos - base_f) as f32;
            let s0 = if base < 0 {
                self.prev
            } else {
                input[base as usize]
            };
            let s1 = input[(base + 1) as usize];
            out.push(s0 * (1.0 - frac) + s1 * frac);
            self.pos += self.step;
        }
        // Shift `pos` so it stays relative to the start of the *next* chunk.
        self.pos -= n as f64;
        self.prev = input[n - 1];
        out
    }
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
