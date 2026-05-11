//! History query/mutation. The history vector lives inside `Config` so that
//! existing on-disk TOML stays compatible with the legacy egui app.

use tauri::{AppHandle, Emitter, State};

use crate::commands::osc_cmd::dispatch;
use crate::config::HistoryEntry;
use crate::state::AppState;

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Vec<HistoryEntry> {
    state
        .config
        .lock()
        .unwrap()
        .history
        .iter()
        .cloned()
        .collect()
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>, app: AppHandle) {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.history.clear();
        cfg.save();
    }
    let _ = app.emit("history-changed", ());
}

/// Re-send a previously sent message at `index` (0 = oldest, len-1 = most
/// recent). Goes through the same `dispatch` pipeline as a fresh send —
/// OSC + TTS speak + push to history (which promotes the entry to the
/// back of the deque on duplicate).
#[tauri::command]
pub fn resend_history(
    index: usize,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    let text = state
        .config
        .lock()
        .unwrap()
        .history
        .get(index)
        .map(|e| e.text.clone())
        .ok_or_else(|| "历史项不存在".to_string())?;
    dispatch(&text, &state, &app)
}
