//! Window-level toggles (always-on-top). Persists to config, applies the
//! change to the live window, and emits `config-changed` so the frontend
//! store re-renders without waiting for the next full reload.

use tauri::{Manager, State, WebviewWindow};

use crate::state::AppState;

#[tauri::command]
pub fn set_always_on_top(
    pinned: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let window: WebviewWindow = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    window
        .set_always_on_top(pinned)
        .map_err(|e| format!("set_always_on_top: {e}"))?;
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.always_on_top = pinned;
    }
    state.save_and_emit(&app);
    Ok(())
}
