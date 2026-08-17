//! Shared application state held in `tauri::State<AppState>`. Every command
//! that touches config, the OSC socket, or the TTS worker reaches through
//! here. Heavy mutexes are kept narrow — lock only for the read/write, not
//! across IO.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::config::Config;
use crate::download::ModelDownloader;
use crate::tts_worker::TtsHandle;

/// How long a hostname-resolution result (success or failure) is trusted
/// before `target()` calls getaddrinfo again.
const DNS_TTL: Duration = Duration::from_secs(30);

/// Cached outcome of the last hostname resolution — see `AppState::target`.
struct ResolvedHost {
    host: String,
    ip: Option<IpAddr>,
    at: Instant,
}

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
    /// Memo for `target()`'s hostname resolution, failures included —
    /// getaddrinfo blocks (for seconds when the name doesn't resolve) and
    /// the typing heartbeat calls `target()` every 1.5 s. IP literals
    /// never touch this.
    resolved_host: Mutex<Option<ResolvedHost>>,
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
            resolved_host: Mutex::new(None),
        })
    }

    pub fn target(&self) -> Option<SocketAddr> {
        let (host, port) = {
            let cfg = self.config.lock().ok()?;
            (cfg.ip.clone(), cfg.port)
        };
        // Fast path: IP literal — the overwhelmingly common case, no DNS.
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Some(SocketAddr::new(ip, port));
        }
        // The settings UI also accepts hostnames ("localhost", "gamingpc.local"
        // for LAN setups). Resolution runs on the send/typing path, so the
        // outcome — including "did not resolve" — is memoised for DNS_TTL;
        // otherwise an unreachable name would block every 1.5 s heartbeat.
        {
            let cache = self.resolved_host.lock().ok()?;
            if let Some(r) = cache.as_ref() {
                if r.host == host && r.at.elapsed() < DNS_TTL {
                    return r.ip.map(|ip| SocketAddr::new(ip, port));
                }
            }
        }
        let ip = resolve_ipv4(&host, port);
        *self.resolved_host.lock().ok()? = Some(ResolvedHost {
            host,
            ip,
            at: Instant::now(),
        });
        ip.map(|ip| SocketAddr::new(ip, port))
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

/// Resolve a hostname to an IPv4 address. IPv6 results are discarded — the
/// OSC socket is bound to `0.0.0.0`, so sending to a v6 target can only
/// fail; treating v6-only names as unresolvable surfaces the clearer
/// "invalid target" error instead of a generic send failure.
fn resolve_ipv4(host: &str, port: u16) -> Option<IpAddr> {
    use std::net::ToSocketAddrs;
    (host, port)
        .to_socket_addrs()
        .ok()?
        .find(|a| a.is_ipv4())
        .map(|a| a.ip())
}
