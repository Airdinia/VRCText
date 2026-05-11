//! Passive presence probe for VRChat's OSC channel.
//!
//! VRChat does not expose a "are you listening?" handshake — but when the
//! user has OSC enabled, it continuously broadcasts avatar parameters and
//! tracking data to its configured *out* port (default `127.0.0.1:9001`).
//! Binding a listener there and observing any packet is therefore strong
//! evidence that OSC is alive on the other side, independent of whether
//! our own `send_to()` calls return Ok (UDP always reports "success"
//! locally regardless of remote state).
//!
//! Emits a `vrc-alive` Tauri event at most every 500 ms. The frontend
//! flips its status indicator to green on receipt and rolls back to idle
//! after ~3 s of silence — so the dot meaningfully reflects "VRChat is
//! talking right now", not "I dispatched a packet just now".

use std::net::UdpSocket;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

/// VRChat's default outbound OSC port. Configurable in VRChat's launch
/// options, but ~all users keep the default. Could be exposed as a
/// config field later if anyone reports needing it.
const VRC_OUT_PORT: u16 = 9001;
const EMIT_THROTTLE: Duration = Duration::from_millis(500);
const RECV_TIMEOUT: Duration = Duration::from_secs(1);

pub fn spawn_listener(app: AppHandle) {
    std::thread::Builder::new()
        .name("vrctext-vrc-listener".into())
        .spawn(move || run(app))
        .ok();
}

fn run(app: AppHandle) {
    // 0.0.0.0 instead of 127.0.0.1 so LAN setups (VRChat on a different
    // machine pointing OSC at us) also get detected. If 9001 is taken by
    // something else on this host, we silently bail — the user just keeps
    // the idle status forever, which is honest given we can't probe.
    let sock = match UdpSocket::bind(("0.0.0.0", VRC_OUT_PORT)) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = sock.set_read_timeout(Some(RECV_TIMEOUT));

    let mut buf = [0u8; 2048];
    let mut last_emit = Instant::now()
        .checked_sub(EMIT_THROTTLE)
        .unwrap_or_else(Instant::now);

    loop {
        match sock.recv_from(&mut buf) {
            Ok(_) => {
                let now = Instant::now();
                if now.duration_since(last_emit) >= EMIT_THROTTLE {
                    let _ = app.emit("vrc-alive", ());
                    last_emit = now;
                }
            }
            Err(_) => {
                // Timeout or transient error — keep the loop alive. We
                // intentionally do NOT emit a "silence" event here; the
                // frontend uses a wall-clock timeout to expire its own
                // "ok" state, which keeps this thread minimal.
                continue;
            }
        }
    }
}
