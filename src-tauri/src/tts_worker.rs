//! Dedicated TTS worker thread. Owns the live `Box<dyn TtsEngine>` for the
//! whole app lifetime; commands arrive on `mpsc`, replies (when needed)
//! ride a oneshot `Sender<R>` packed into the command variant.
//!
//! Why a thread instead of a tokio task:
//!   1. SAPI's `ISpVoice` requires `COINIT_APARTMENTTHREADED` — a single STA
//!      apartment for the lifetime of the COM object.
//!   2. cpal `Stream` is `!Send`; building one in tokio would not survive
//!      the executor moving the future to a different worker.
//!   3. Sherpa synth is CPU-heavy enough to want its own scheduler context
//!      so it doesn't block the runtime.

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::Engine;
use crate::tts::{
    load_sherpa_async, Choice, LoadedSherpa, LoadingEngine, SapiEngine, SherpaEngine, TtsEngine,
};

/// Snapshot exposed to the frontend via the `tts_status` command and the
/// `tts-status` event. Cheaper to keep this as a plain copy-on-read struct
/// than to reach through the worker channel for every UI render.
#[derive(Clone, Serialize, Default)]
pub struct TtsStatus {
    pub engine: &'static str, // "sapi" | "sherpa" | "loading"
    pub available: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub current_device: Option<String>,
    pub current_voice: Option<String>,
}

pub enum TtsCmd {
    Speak(String),
    Stop,
    /// `enabled` mirrors `Config::tts_enabled`. Disabling stops any active
    /// playback; enabling triggers an engine `reload()` AND re-applies the
    /// saved device/voice — `reload()` acquires a fresh OS handle but
    /// returns no routing, so without the re-apply the engine plays into
    /// the system default instead of the user's chosen output.
    SetEnabled {
        enabled: bool,
        device: Option<String>,
        voice: Option<String>,
    },
    SwitchEngine {
        to: Engine,
        device: Option<String>,
        voice: Option<String>,
    },
    ListDevices(Sender<Vec<Choice>>),
    ListVoices(Sender<Vec<Choice>>),
    ApplyDevice {
        key: Option<String>,
        reply: Sender<Option<String>>,
    },
    ApplyVoice {
        key: Option<String>,
        reply: Sender<Option<String>>,
    },
    /// Re-attempt sherpa load. Used after a model download completes.
    ReloadSherpa {
        device: Option<String>,
        voice: Option<String>,
    },
}

pub struct TtsHandle {
    pub tx: Sender<TtsCmd>,
    pub status: Arc<Mutex<TtsStatus>>,
}

pub fn spawn(
    app: AppHandle,
    initial_engine: Engine,
    device: Option<String>,
    voice: Option<String>,
) -> TtsHandle {
    let status = Arc::new(Mutex::new(TtsStatus {
        engine: "sapi",
        ..Default::default()
    }));
    let status_clone = status.clone();

    let (tx, rx) = mpsc::channel::<TtsCmd>();
    thread::Builder::new()
        .name("vrctext-tts".into())
        .spawn(move || {
            run(app, status_clone, rx, initial_engine, device, voice);
        })
        .expect("spawn tts worker");

    TtsHandle { tx, status }
}

fn run(
    app: AppHandle,
    status: Arc<Mutex<TtsStatus>>,
    rx: mpsc::Receiver<TtsCmd>,
    initial_engine: Engine,
    initial_device: Option<String>,
    initial_voice: Option<String>,
) {
    // Establish an STA COM apartment for SAPI. Must precede any SAPI calls.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let mut engine: Box<dyn TtsEngine> = Box::new(SapiEngine::new());
    let mut sherpa_load: Option<mpsc::Receiver<Result<LoadedSherpa, String>>> = None;
    let mut current_engine: Engine = Engine::Sapi;
    // Device to bind once an in-flight sherpa load completes. There is no
    // `pending_voice` counterpart: the voice preference is handed to
    // `load_sherpa_async` itself, which resolves it during the load.
    let mut pending_device: Option<String> = None;

    if matches!(initial_engine, Engine::Sapi) {
        // Apply persisted device/voice once and use the resolved keys for
        // the initial status snapshot.
        let cur_dev = engine.apply_device(initial_device.as_deref().filter(|s| !s.is_empty()));
        let cur_voice = engine.apply_voice(initial_voice.as_deref().filter(|s| !s.is_empty()));
        publish_status(
            &app,
            &status,
            "sapi",
            engine.available(),
            false,
            engine.unavailable_detail(),
            cur_dev,
            cur_voice,
        );
    } else {
        // Engine::Sherpa: kick off async load, transient LoadingEngine in the meantime.
        engine = Box::new(LoadingEngine::loading());
        sherpa_load = Some(load_sherpa_async(initial_voice));
        pending_device = initial_device;
        current_engine = Engine::Sherpa;
        publish_status(&app, &status, "loading", false, true, None, None, None);
    }

    loop {
        // Drain any sherpa load result first.
        if let Some(rx_load) = &sherpa_load {
            match rx_load.try_recv() {
                Ok(Ok(loaded)) => {
                    // The async loader already resolved `pending_voice` (or
                    // fell back to the first installed pack) — re-applying
                    // the saved voice here would only trigger a redundant
                    // synchronous model reload when the key didn't resolve.
                    let cur_voice = loaded.voice_key.clone();
                    let mut new_engine = SherpaEngine::from_loaded(loaded);
                    let cur_dev = new_engine.apply_device(pending_device.as_deref());
                    let avail = new_engine.available();
                    let detail = new_engine.unavailable_detail();
                    engine = Box::new(new_engine);
                    sherpa_load = None;
                    publish_status(
                        &app, &status, "sherpa", avail, false, detail, cur_dev, cur_voice,
                    );
                }
                Ok(Err(msg)) => {
                    // Replace the LoadingEngine placeholder with one that
                    // permanently carries the failure reason — otherwise
                    // any subsequent publish_status (e.g. on a Volume
                    // toggle) would read back "AI 引擎加载中…" from the
                    // stale LoadingEngine and look like the load is still
                    // in flight.
                    sherpa_load = None;
                    engine = Box::new(LoadingEngine::failed(msg.clone()));
                    publish_status(&app, &status, "sherpa", false, false, Some(msg), None, None);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    sherpa_load = None;
                }
            }
        }

        match rx.recv_timeout(Duration::from_millis(80)) {
            Ok(cmd) => match cmd {
                TtsCmd::Speak(text) => engine.speak(&text),
                TtsCmd::Stop => engine.stop(),
                TtsCmd::SetEnabled {
                    enabled: true,
                    device,
                    voice,
                } => {
                    // Re-acquire OS audio handles in case audiosrv was
                    // restarted while idle, then re-bind the saved routing
                    // (reload() returns a virgin engine with no device).
                    engine.reload();
                    let cur_dev = engine.apply_device(device.as_deref());
                    let cur_voice = engine.apply_voice(voice.as_deref());
                    let avail = engine.available();
                    let detail = engine.unavailable_detail();
                    // If a sherpa load is still in flight, keep reporting
                    // the loading state — otherwise the LoadingEngine's
                    // "@i18n:engineLoading" detail renders as an error.
                    let loading = sherpa_load.is_some();
                    publish_status(
                        &app,
                        &status,
                        engine_label(current_engine, loading),
                        avail,
                        loading,
                        detail,
                        cur_dev,
                        cur_voice,
                    );
                }
                TtsCmd::SetEnabled { enabled: false, .. } => {
                    engine.stop();
                }
                TtsCmd::SwitchEngine { to, device, voice } => {
                    engine.stop();
                    current_engine = to;
                    match to {
                        Engine::Sapi => {
                            engine = Box::new(SapiEngine::new());
                            let cur_dev = engine.apply_device(device.as_deref());
                            let cur_voice = engine.apply_voice(voice.as_deref());
                            sherpa_load = None;
                            pending_device = None;
                            publish_status(
                                &app,
                                &status,
                                "sapi",
                                engine.available(),
                                false,
                                engine.unavailable_detail(),
                                cur_dev,
                                cur_voice,
                            );
                        }
                        Engine::Sherpa => {
                            engine = Box::new(LoadingEngine::loading());
                            sherpa_load = Some(load_sherpa_async(voice));
                            pending_device = device;
                            publish_status(&app, &status, "loading", false, true, None, None, None);
                        }
                    }
                }
                TtsCmd::ListDevices(reply) => {
                    let _ = reply.send(engine.device_choices());
                }
                TtsCmd::ListVoices(reply) => {
                    let _ = reply.send(engine.voice_choices());
                }
                TtsCmd::ApplyDevice { key, reply } => {
                    // A load in flight means the live engine is a
                    // placeholder — remember the pick so the completion
                    // path binds it instead of the stale one.
                    if sherpa_load.is_some() {
                        pending_device = key.clone();
                    }
                    let resolved = engine.apply_device(key.as_deref());
                    {
                        let mut s = status.lock().unwrap();
                        s.current_device = resolved.clone();
                    }
                    let _ = reply.send(resolved);
                    publish_only_event(&app, &status);
                }
                TtsCmd::ApplyVoice { key, reply } => {
                    // Restart an in-flight load with the new voice — the
                    // placeholder engine can't apply it, and letting the
                    // old load finish would resurrect the previous
                    // speaker. Dropping the old receiver discards its
                    // result harmlessly.
                    if sherpa_load.is_some() {
                        sherpa_load = Some(load_sherpa_async(key.clone()));
                    }
                    let resolved = engine.apply_voice(key.as_deref());
                    {
                        let mut s = status.lock().unwrap();
                        s.current_voice = resolved.clone();
                    }
                    let _ = reply.send(resolved);
                    publish_only_event(&app, &status);
                }
                TtsCmd::ReloadSherpa { device, voice } => {
                    if matches!(current_engine, Engine::Sherpa) {
                        engine = Box::new(LoadingEngine::loading());
                        sherpa_load = Some(load_sherpa_async(voice));
                        pending_device = device;
                        publish_status(&app, &status, "loading", false, true, None, None, None);
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout is the polling loop for sherpa_load.
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}

fn engine_label(engine: Engine, loading: bool) -> &'static str {
    if loading {
        return "loading";
    }
    match engine {
        Engine::Sapi => "sapi",
        Engine::Sherpa => "sherpa",
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_status(
    app: &AppHandle,
    status: &Arc<Mutex<TtsStatus>>,
    engine: &'static str,
    available: bool,
    loading: bool,
    error: Option<String>,
    current_device: Option<String>,
    current_voice: Option<String>,
) {
    {
        let mut s = status.lock().unwrap();
        s.engine = engine;
        s.available = available;
        s.loading = loading;
        s.error = error;
        // Unconditional: every caller passes the engine's true current
        // routing. Keeping the old value on `None` used to leave a stale
        // device/voice name displayed after engine switches.
        s.current_device = current_device;
        s.current_voice = current_voice;
    }
    publish_only_event(app, status);
}

fn publish_only_event(app: &AppHandle, status: &Arc<Mutex<TtsStatus>>) {
    let snap = status.lock().unwrap().clone();
    let _ = app.emit("tts-status", snap);
}
