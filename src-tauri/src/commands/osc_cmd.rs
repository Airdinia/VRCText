//! OSC send + typing-indicator commands. Mirrors the legacy `dispatch`
//! pipeline: OSC → TTS (when enabled) → push to history → emit event.
//! The TTS step is dispatched through the worker thread so this command
//! returns immediately and the synthesis runs off the IPC path.

use tauri::{AppHandle, Emitter, State};

use crate::osc;
use crate::state::AppState;
use crate::tts_worker::TtsCmd;

/// OSC send + (optional) TTS speak + push to history. Returns Ok even if
/// only one of the two channels succeeded — caller surfaces partial
/// success in the toast text.
pub fn dispatch(
    text: &str,
    state: &AppState,
    app: &AppHandle,
) -> Result<(), String> {
    let target = state.target().ok_or_else(|| "@i18n:errOscTarget".to_string())?;
    let (play_sound, tts_enabled) = {
        let cfg = state.config.lock().unwrap();
        (cfg.play_sound, cfg.tts_enabled)
    };
    let pkt = osc::encode_chatbox_input(text, true, play_sound);
    {
        let socket = state.socket.lock().unwrap();
        osc::send(&socket, target, &pkt).map_err(|_| "@i18n:errOscSend".to_string())?;
        // Match legacy: stop the typing indicator on a successful send even
        // if the heartbeat is still scheduled.
        let typing = osc::encode_chatbox_typing(false);
        let _ = osc::send(&socket, target, &typing);
    }
    *state.typing_active.lock().unwrap() = false;

    // Fire-and-forget TTS speak. The worker handles purge-before-speak via
    // SPF_PURGEBEFORESPEAK (SAPI) / gen_counter bump (Sherpa), so a quick
    // burst of sends preempts each other rather than queuing.
    if tts_enabled {
        let _ = state.tts.tx.send(TtsCmd::Speak(text.to_string()));
    }

    // Push to history + persist + notify frontend.
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.push_history(text.to_string());
        cfg.save();
    }
    let _ = app.emit("history-changed", ());
    Ok(())
}

#[tauri::command]
pub fn send_message(
    text: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("@i18n:errEmptyMessage".into());
    }
    dispatch(trimmed, &state, &app)
}

#[tauri::command]
pub fn set_typing(typing: bool, state: State<'_, AppState>) -> Result<(), String> {
    let target = match state.target() {
        Some(t) => t,
        None => return Ok(()),
    };
    let pkt = osc::encode_chatbox_typing(typing);
    let socket = state.socket.lock().unwrap();
    let _ = osc::send(&socket, target, &pkt);
    *state.typing_active.lock().unwrap() = typing;
    Ok(())
}
