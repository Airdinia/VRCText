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
        let (host, port) = {
            let cfg = self.config.lock().ok()?;
            (cfg.ip.clone(), cfg.port)
        };
        // Fast path: IP literal — the overwhelmingly common case, no DNS.
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return Some(std::net::SocketAddr::new(ip, port));
        }
        // The settings UI also accepts hostnames ("localhost", "gamingpc.local"
        // for LAN setups) — resolve through the OS. Prefer IPv4: our socket is
        // bound v4 and VRChat listens on the v4 stack. The OS caches lookups,
        // so the per-send cost for hostname users is negligible.
        use std::net::ToSocketAddrs;
        let addrs = (host.as_str(), port).to_socket_addrs().ok()?;
        let mut fallback = None;
        for a in addrs {
            if a.is_ipv4() {
                return Some(a);
            }
            fallback.get_or_insert(a);
        }
        fallback
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
