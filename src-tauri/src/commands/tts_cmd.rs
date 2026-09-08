//! TTS commands. Every call funnels through the dedicated TTS worker
//! thread (see `tts_worker.rs`); commands that need a return value use a
//! oneshot mpsc channel and `spawn_blocking` to await without parking the
//! tokio runtime.

use std::sync::mpsc;
use std::time::Duration;

use tauri::{AppHandle, State};

use crate::config::Engine;
use crate::state::AppState;
use crate::tts::Choice;
use crate::tts_worker::{TtsCmd, TtsStatus};

const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

#[tauri::command]
pub fn tts_status(state: State<'_, AppState>) -> TtsStatus {
    state.tts.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn tts_speak(text: String, state: State<'_, AppState>) -> Result<(), String> {
    let text = crate::osc::validate_message(&text)?.to_string();
    state
        .tts
        .tx
        .send(TtsCmd::Speak(text))
        .map_err(|_| "tts worker dead".to_string())
}

#[tauri::command]
pub fn tts_stop(state: State<'_, AppState>) -> Result<(), String> {
    state
        .tts
        .tx
        .send(TtsCmd::Stop)
        .map_err(|_| "tts worker dead".to_string())
}

#[tauri::command]
pub fn tts_set_enabled(
    enabled: bool,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (device, voice) = {
        let mut cfg = state.config.lock().unwrap();
        cfg.tts_enabled = enabled;
        (
            cfg.current_device().map(|s| s.to_string()),
            cfg.current_voice().map(|s| s.to_string()),
        )
    };
    state.save_and_emit(&app);
    state
        .tts
        .tx
        .send(TtsCmd::SetEnabled {
            enabled,
            device,
            voice,
        })
        .map_err(|_| "tts worker dead".to_string())
}

#[tauri::command]
pub fn tts_switch_engine(
    engine: Engine,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let (device, voice) = {
        let mut cfg = state.config.lock().unwrap();
        cfg.engine = engine;
        (
            cfg.current_device().map(|s| s.to_string()),
            cfg.current_voice().map(|s| s.to_string()),
        )
    };
    state.save_and_emit(&app);
    state
        .tts
        .tx
        .send(TtsCmd::SwitchEngine {
            to: engine,
            device,
            voice,
        })
        .map_err(|_| "tts worker dead".to_string())
}

#[tauri::command]
pub async fn tts_list_devices(state: State<'_, AppState>) -> Result<Vec<Choice>, String> {
    let (tx, rx) = mpsc::channel();
    state
        .tts
        .tx
        .send(TtsCmd::ListDevices(tx))
        .map_err(|_| "tts worker dead".to_string())?;
    tokio::task::spawn_blocking(move || rx.recv_timeout(REPLY_TIMEOUT))
        .await
        .map_err(|e| format!("tts join: {e}"))?
        .map_err(|_| "tts no reply".to_string())
}

#[tauri::command]
pub async fn tts_list_voices(state: State<'_, AppState>) -> Result<Vec<Choice>, String> {
    let (tx, rx) = mpsc::channel();
    state
        .tts
        .tx
        .send(TtsCmd::ListVoices(tx))
        .map_err(|_| "tts worker dead".to_string())?;
    tokio::task::spawn_blocking(move || rx.recv_timeout(REPLY_TIMEOUT))
        .await
        .map_err(|e| format!("tts join: {e}"))?
        .map_err(|_| "tts no reply".to_string())
}

#[tauri::command]
pub async fn tts_apply_device(
    key: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let (tx, rx) = mpsc::channel();
    state
        .tts
        .tx
        .send(TtsCmd::ApplyDevice {
            key: key.clone(),
            reply: tx,
        })
        .map_err(|_| "tts worker dead".to_string())?;
    let (applied_engine, resolved) =
        tokio::task::spawn_blocking(move || rx.recv_timeout(REPLY_TIMEOUT))
            .await
            .map_err(|e| format!("tts join: {e}"))?
            .map_err(|_| "tts no reply".to_string())?;
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.set_device_for(applied_engine, resolved.clone().or(key));
        cfg.save();
    }
    Ok(resolved)
}

#[tauri::command]
pub async fn tts_apply_voice(
    key: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let (tx, rx) = mpsc::channel();
    state
        .tts
        .tx
        .send(TtsCmd::ApplyVoice {
            key: key.clone(),
            reply: tx,
        })
        .map_err(|_| "tts worker dead".to_string())?;
    let (applied_engine, resolved) =
        tokio::task::spawn_blocking(move || rx.recv_timeout(REPLY_TIMEOUT))
            .await
            .map_err(|e| format!("tts join: {e}"))?
            .map_err(|_| "tts no reply".to_string())?;
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.set_voice_for(applied_engine, resolved.clone().or(key));
        cfg.save();
    }
    Ok(resolved)
}
