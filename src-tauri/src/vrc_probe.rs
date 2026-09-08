//! Passive activity indicator for VRChat's default outbound OSC port.
//! This observes traffic, not acknowledgement of chat delivery.
//! Keep receiving while hidden; only throttle frontend events.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const VRC_OUT_PORT: u16 = 9001;
const EMIT_THROTTLE: Duration = Duration::from_secs(1);
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct ProbeHandle {
    visible: Arc<AtomicBool>,
}
impl ProbeHandle {
    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Relaxed);
    }
}
pub fn spawn_listener(app: AppHandle) -> Option<ProbeHandle> {
    let visible = Arc::new(AtomicBool::new(true));
    let visible_for_thread = visible.clone();
    thread::Builder::new()
        .name("vrctext-vrc-listener".into())
        .spawn(move || run(app, visible_for_thread))
        .ok()?;
    Some(ProbeHandle { visible })
}
fn run(app: AppHandle, visible: Arc<AtomicBool>) {
    // Also observe LAN setups. If another OSC app owns this port,
    // leave the indicator idle; sending chat remains independent.
    let sock = match UdpSocket::bind(("0.0.0.0", VRC_OUT_PORT)) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = sock.set_read_timeout(Some(RECV_TIMEOUT));
    let mut last_emit = Instant::now() - EMIT_THROTTLE;
    let mut buf = [0u8; 2048];
    loop {
        if sock.recv_from(&mut buf).is_ok()
            && visible.load(Ordering::Relaxed)
            && last_emit.elapsed() >= EMIT_THROTTLE
        {
            let _ = app.emit("vrc-alive", ());
            last_emit = Instant::now();
        }
    }
}
