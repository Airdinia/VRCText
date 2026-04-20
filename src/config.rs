use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

pub const HISTORY_CAP: usize = 50;

/// Which TTS backend to drive. `Sapi` is the zero-dep default; `Sherpa` is a
/// placeholder for the upcoming ONNX-based AI engine (GPU by default).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Sapi,
    Sherpa,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub text: String,
    pub ts: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub ip: String,
    pub port: u16,
    pub play_sound: bool,
    pub always_on_top: bool,
    #[serde(default)]
    pub tts_enabled: bool,
    #[serde(default)]
    pub engine: Engine,
    /// SAPI's remembered device. The `alias` keeps old configs readable
    /// after the per-engine split — before the AI backend landed there was
    /// only one TTS engine, so the legacy field is SAPI by convention.
    #[serde(default, alias = "tts_device_name")]
    pub tts_device_sapi: Option<String>,
    #[serde(default, alias = "tts_voice_name")]
    pub tts_voice_sapi: Option<String>,
    #[serde(default)]
    pub tts_device_sherpa: Option<String>,
    #[serde(default)]
    pub tts_voice_sherpa: Option<String>,
    #[serde(default)]
    pub history: VecDeque<HistoryEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".into(),
            port: 9000,
            play_sound: true,
            always_on_top: false,
            tts_enabled: false,
            engine: Engine::default(),
            tts_device_sapi: None,
            tts_voice_sapi: None,
            tts_device_sherpa: None,
            tts_voice_sherpa: None,
            history: VecDeque::new(),
        }
    }
}

impl Config {
    /// Device name saved for whichever engine is currently selected. Keeps
    /// the two engines' picks independent so swapping doesn't clobber.
    pub fn current_device(&self) -> Option<&str> {
        match self.engine {
            Engine::Sapi => self.tts_device_sapi.as_deref(),
            Engine::Sherpa => self.tts_device_sherpa.as_deref(),
        }
    }

    pub fn current_voice(&self) -> Option<&str> {
        match self.engine {
            Engine::Sapi => self.tts_voice_sapi.as_deref(),
            Engine::Sherpa => self.tts_voice_sherpa.as_deref(),
        }
    }

    pub fn set_current_device(&mut self, value: Option<String>) {
        match self.engine {
            Engine::Sapi => self.tts_device_sapi = value,
            Engine::Sherpa => self.tts_device_sherpa = value,
        }
    }

    pub fn set_current_voice(&mut self, value: Option<String>) {
        match self.engine {
            Engine::Sapi => self.tts_voice_sapi = value,
            Engine::Sherpa => self.tts_voice_sherpa = value,
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "vrctext", "vrctext")?;
    Some(dirs.config_dir().join("config.toml"))
}

/// Where AI-engine model bundles are expected to live. Today users drop
/// extracted archives here manually; a future commit will add an in-app
/// downloader that populates this same path.
pub fn models_dir() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "vrctext", "vrctext")?;
    Some(dirs.data_dir().join("models"))
}

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

impl Config {
    pub fn load() -> Self {
        let Some(path) = config_path() else { return Self::default(); };
        let Ok(raw) = fs::read_to_string(&path) else { return Self::default(); };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = config_path() else { return; };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = fs::write(path, s);
        }
    }

    pub fn push_history(&mut self, msg: String) {
        let ts = now_ts();
        if self
            .history
            .back()
            .map(|e| e.text.as_str())
            .map_or(false, |t| t == msg.as_str())
        {
            if let Some(e) = self.history.back_mut() {
                e.ts = ts;
            }
            return;
        }
        self.history.push_back(HistoryEntry { text: msg, ts });
        while self.history.len() > HISTORY_CAP {
            self.history.pop_front();
        }
    }
}
