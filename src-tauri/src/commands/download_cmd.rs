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
use tauri::{AppHandle, Emitter, State};

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
        "matcha" => Some(ModelKind::MatchaZhBaker),
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

    {
        let mut active = state.download.lock().unwrap();
        if let Some(prev) = active.as_ref() {
            let snap = prev.snapshot();
            if !snap.done {
                return Err("@i18n:dlAlreadyRunning".into());
            }
        }
        let downloader = ModelDownloader::start(k, dir);
        *active = Some(downloader);
    }
    let state_arc = state.download_arc();
    let tts_tx = state.tts.tx.clone();
    let cur_engine = state.config.lock().unwrap().engine;
    let cur_device = state
        .config
        .lock()
        .unwrap()
        .tts_device_sherpa
        .clone();
    let cur_voice = state
        .config
        .lock()
        .unwrap()
        .tts_voice_sherpa
        .clone();

    // Poll the downloader on a watcher thread so the calling Tauri command
    // can return immediately. The downloader itself runs on its own thread
    // (spawned inside `ModelDownloader::start`) — we only watch its state.
    thread::spawn(move || watch(state_arc, k, app, tts_tx, cur_engine, cur_device, cur_voice));
    Ok(())
}

fn watch(
    state: Arc<Mutex<Option<ModelDownloader>>>,
    kind: ModelKind,
    app: AppHandle,
    tts_tx: std::sync::mpsc::Sender<TtsCmd>,
    cur_engine: Engine,
    cur_device: Option<String>,
    cur_voice: Option<String>,
) {
    let mut last_status = String::new();
    let mut last_progress: f32 = -1.0;
    loop {
        let snap: DownloadState = match state.lock().unwrap().as_ref() {
            Some(d) => d.snapshot(),
            None => return,
        };
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
                if matches!(cur_engine, Engine::Sherpa) {
                    let _ = tts_tx.send(TtsCmd::ReloadSherpa {
                        device: cur_device,
                        voice: cur_voice,
                    });
                }
            }
            // Drop the downloader so the next download_pack can start.
            *state.lock().unwrap() = None;
            return;
        }
        thread::sleep(Duration::from_millis(200));
    }
}

#[tauri::command]
pub fn delete_models() -> Result<(), String> {
    let dir = models_dir().ok_or_else(|| "no models dir".to_string())?;
    if !dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("@i18n:dlDeleteFail|{e}"))?;
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
