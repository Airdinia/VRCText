//! Config load + partial save. The frontend never writes TOML directly;
//! every mutation is funneled through `save_partial_config` so the Rust
//! side stays the sole owner of the schema and the on-disk bytes.

use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};

use crate::config::{Config, Engine};
use crate::state::AppState;

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

/// Every field is optional — only the ones present in the patch are
/// applied. Returns the resulting full config so the frontend can refresh
/// its store from a single round-trip.
#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ConfigPatch {
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub play_sound: Option<bool>,
    pub always_on_top: Option<bool>,
    pub tts_enabled: Option<bool>,
    pub engine: Option<Engine>,
    pub tts_device_sapi: Option<Option<String>>,
    pub tts_voice_sapi: Option<Option<String>>,
    pub tts_device_sherpa: Option<Option<String>>,
    pub tts_voice_sherpa: Option<Option<String>>,
    pub language: Option<String>,
}

#[tauri::command]
pub fn save_partial_config(
    patch: ConfigPatch,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Config {
    let snapshot = {
        let mut cfg = state.config.lock().unwrap();
        if let Some(v) = patch.ip {
            cfg.ip = v;
        }
        if let Some(v) = patch.port {
            cfg.port = v;
        }
        if let Some(v) = patch.play_sound {
            cfg.play_sound = v;
        }
        if let Some(v) = patch.always_on_top {
            cfg.always_on_top = v;
        }
        if let Some(v) = patch.tts_enabled {
            cfg.tts_enabled = v;
        }
        if let Some(v) = patch.engine {
            cfg.engine = v;
        }
        if let Some(v) = patch.tts_device_sapi {
            cfg.tts_device_sapi = v;
        }
        if let Some(v) = patch.tts_voice_sapi {
            cfg.tts_voice_sapi = v;
        }
        if let Some(v) = patch.tts_device_sherpa {
            cfg.tts_device_sherpa = v;
        }
        if let Some(v) = patch.tts_voice_sherpa {
            cfg.tts_voice_sherpa = v;
        }
        if let Some(v) = patch.language {
            cfg.language = v;
        }
        cfg.save();
        cfg.clone()
    };
    let _ = app.emit("config-changed", &snapshot);
    snapshot
}
