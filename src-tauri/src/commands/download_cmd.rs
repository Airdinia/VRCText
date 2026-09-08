//! Model-pack download commands. The downloader spawns its own thread; we
//! poll its `DownloadState` every 200ms and forward changes to the
//! frontend via `download-progress` / `download-complete` / `download-error`
//! events. Once a download finishes, we kick the TTS worker to retry the
//! Sherpa load if the user is currently on the Sherpa engine.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::config::{models_dir, Engine};
use crate::download::{installed_kinds, DownloadState, ModelDownloader, ModelKind};
use crate::state::AppState;
use crate::tts_worker::TtsCmd;

#[derive(Clone, Serialize)]
pub struct DownloadProgressEvent {
    pub kind: &'static str,
    pub status: String,
    pub progress: f32,
}

#[derive(Clone, Serialize)]
pub struct DownloadErrorEvent {
    pub kind: &'static str,
    pub message: String,
}

fn kind_from_str(s: &str) -> Option<ModelKind> {
    match s {
        "kokoro" => Some(ModelKind::KokoroMultiLang),
        _ => None,
    }
}

fn kind_str(k: ModelKind) -> &'static str {
    match k {
        ModelKind::MatchaZhBaker => "matcha",
        ModelKind::KokoroMultiLang => "kokoro",
    }
}

#[tauri::command]
pub fn download_pack(
    kind: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let k = kind_from_str(&kind).ok_or_else(|| format!("unknown pack: {kind}"))?;
    let dir: PathBuf = models_dir().ok_or_else(|| "no models dir".to_string())?;

    let dl_state = {
        let mut active = state.download.lock().unwrap();
        if active.is_some() {
            return Err("@i18n:dlAlreadyRunning".into());
        }
        let downloader = ModelDownloader::start(k, dir);
        let dl_state = downloader.state.clone();
        *active = Some(downloader);
        dl_state
    };
    let slot = state.download_arc();
    let tts_tx = state.tts.tx.clone();

    // Poll the downloader on a watcher thread so the calling Tauri command
    // can return immediately. The downloader itself runs on its own thread
    // (spawned inside `ModelDownloader::start`) — we only watch its state.
    thread::spawn(move || watch(slot, dl_state, k, app, tts_tx));
    Ok(())
}

fn watch(
    slot: Arc<Mutex<Option<ModelDownloader>>>,
    // The watched download's own state. Polling this instead of whatever
    // currently sits in `slot` keeps a lingering watcher from observing —
    // or, worse, clearing — a newer download that replaced its own.
    dl_state: Arc<Mutex<DownloadState>>,
    kind: ModelKind,
    app: AppHandle,
    tts_tx: std::sync::mpsc::Sender<TtsCmd>,
) {
    let mut last_status = String::new();
    let mut last_progress: f32 = -1.0;
    loop {
        let snap: DownloadState = dl_state.lock().unwrap().clone();
        // Throttle: emit only when status text or 0.5%+ progress change.
        if snap.status != last_status || (snap.progress - last_progress).abs() >= 0.005 {
            let _ = app.emit(
                "download-progress",
                DownloadProgressEvent {
                    kind: kind_str(kind),
                    status: snap.status.clone(),
                    progress: snap.progress,
                },
            );
            last_status = snap.status;
            last_progress = snap.progress;
        }
        if snap.done {
            // Keep deletion/new downloads serialized until completion has
            // queued its reload, using settings current at completion time.
            let mut active = slot.lock().unwrap();
            if let Some(msg) = snap.error {
                let _ = app.emit(
                    "download-error",
                    DownloadErrorEvent {
                        kind: kind_str(kind),
                        message: msg,
                    },
                );
            } else {
                let _ = app.emit("download-complete", kind_str(kind));
                // If the user is on Sherpa engine, retry loading the model
                // now that the bytes are on disk.
                let state = app.state::<AppState>();
                let cfg = state.config.lock().unwrap();
                if matches!(cfg.engine, Engine::Sherpa) {
                    let _ = tts_tx.send(TtsCmd::ReloadSherpa {
                        device: cfg.tts_device_sherpa.clone(),
                        voice: cfg.tts_voice_sherpa.clone(),
                    });
                }
            }
            // Free the slot for the next download — but only if it still
            // holds this download, not a newer one that already took over.
            if active
                .as_ref()
                .is_some_and(|d| Arc::ptr_eq(&d.state, &dl_state))
            {
                *active = None;
            }
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

#[tauri::command]
pub fn delete_models(state: State<'_, AppState>) -> Result<(), String> {
    // Refuse while a download is writing into the directory — deleting
    // under it would corrupt the extract and race the verifier. The guard
    // stays held across the removal (up to a couple of seconds for the
    // Kokoro pack) so a concurrent download_pack serialises behind the
    // delete instead of starting to write mid-teardown.
    let guard = state.download.lock().unwrap();
    if guard.is_some() {
        return Err("@i18n:dlAlreadyRunning".into());
    }
    let dir = models_dir().ok_or_else(|| "no models dir".to_string())?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("@i18n:dlDeleteFail|{e}"))?;
    }
    drop(guard);
    // The live engine may still hold the deleted pack in RAM and keep
    // reporting "available" (preview would happily play from memory).
    // Kick a reload so status honestly flips to "no model — download below".
    let (engine, device, voice) = {
        let cfg = state.config.lock().unwrap();
        (
            cfg.engine,
            cfg.tts_device_sherpa.clone(),
            cfg.tts_voice_sherpa.clone(),
        )
    };
    if matches!(engine, Engine::Sherpa) {
        let _ = state.tts.tx.send(TtsCmd::ReloadSherpa { device, voice });
    }
    Ok(())
}

#[tauri::command]
pub fn installed_packs() -> Vec<&'static str> {
    let dir = match models_dir() {
        Some(d) => d,
        None => return Vec::new(),
    };
    installed_kinds(&dir).into_iter().map(kind_str).collect()
}
