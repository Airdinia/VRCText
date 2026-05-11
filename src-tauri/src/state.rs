//! Shared application state held in `tauri::State<AppState>`. Every command
//! that touches config, the OSC socket, or the TTS worker reaches through
//! here. Heavy mutexes are kept narrow — lock only for the read/write, not
//! across IO.

use std::net::UdpSocket;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter};

use crate::config::Config;
use crate::download::ModelDownloader;
use crate::tts_worker::TtsHandle;

pub struct AppState {
    pub config: Mutex<Config>,
    pub socket: Mutex<UdpSocket>,
    /// Whether the typing-indicator heartbeat is currently active. The
    /// frontend drives the cadence; we keep the latest value so that a send
    /// can issue an explicit `/chatbox/typing false` and reset us.
    pub typing_active: Mutex<bool>,
    pub tts: TtsHandle,
    /// Active model downloader. Wrapped in `Arc<Mutex<...>>` so the
    /// download_cmd watcher thread can hold a reference for the duration of
    /// the download without racing the AppState lock.
    pub download: Arc<Mutex<Option<ModelDownloader>>>,
}

impl AppState {
    pub fn new(tts: TtsHandle) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        crate::win::disable_udp_connreset(&socket);
        Ok(Self {
            config: Mutex::new(Config::load()),
            socket: Mutex::new(socket),
            typing_active: Mutex::new(false),
            tts,
            download: Arc::new(Mutex::new(None)),
        })
    }

    pub fn target(&self) -> Option<std::net::SocketAddr> {
        let cfg = self.config.lock().ok()?;
        let ip: std::net::IpAddr = cfg.ip.parse().ok()?;
        Some(std::net::SocketAddr::new(ip, cfg.port))
    }

    /// Convenience for the watcher thread that polls `ModelDownloader::snapshot()`.
    pub fn download_arc(&self) -> Arc<Mutex<Option<ModelDownloader>>> {
        self.download.clone()
    }

    /// Save current config to disk and notify the frontend so its mirror
    /// store re-renders. Use after every backend-side mutation that
    /// doesn't already go through `save_partial_config`.
    pub fn save_and_emit(&self, app: &AppHandle) {
        let snapshot = {
            let cfg = self.config.lock().unwrap();
            cfg.save();
            cfg.clone()
        };
        let _ = app.emit("config-changed", &snapshot);
    }
}
