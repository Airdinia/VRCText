use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const HISTORY_CAP: usize = 50;

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
    pub tts_device_name: Option<String>,
    #[serde(default)]
    pub tts_voice_name: Option<String>,
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
            tts_device_name: None,
            tts_voice_name: None,
            history: VecDeque::new(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("dev", "vrctext", "vrctext")?;
    Some(dirs.config_dir().join("config.toml"))
}

pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
