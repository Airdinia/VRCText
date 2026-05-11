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
//! Emits a `vrc-alive` Tauri event at most every 1 s. The frontend flips
//! its status indicator to green on receipt and rolls back to idle after
//! ~3 s of silence — so the dot meaningfully reflects "VRChat is talking
//! right now", not "I dispatched a packet just now".
//!
//! Self-throttles two ways to stay near-zero CPU when nothing useful is
//! happening:
//!   1. After each emit, `thread::sleep(EMIT_THROTTLE)` — VRChat sends at
//!      ~60 Hz so a naive "wake on every packet" loop spins 60 syscalls/s.
//!      Sleeping lets the OS socket buffer absorb intermediate packets;
//!      the next `recv_from` returns instantly with one of them.
//!   2. When the parent window is minimised the user can't see the
//!      indicator anyway — the listener thread parks until the window
//!      is restored. See `ProbeHandle::set_visible` / `lib.rs`.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, Thread};
use std::time::Duration;

use tauri::{AppHandle, Emitter};

/// VRChat's default outbound OSC port. Configurable in VRChat's launch
/// options, but ~all users keep the default.
const VRC_OUT_PORT: u16 = 9001;
/// Lower bound between consecutive `vrc-alive` emits. Front-end keeps the
/// "ok" dot lit for STATUS_HOLD_MS = 3 s, so 1 s gives 3× headroom against
/// dropped packets.
const EMIT_THROTTLE: Duration = Duration::from_secs(1);
/// Idle-loop timeout — only triggers when the socket buffer is empty
/// (i.e. VRChat isn't running). Long enough that an unused install
/// barely consumes CPU; short enough that the first packet after VRChat
/// starts is observed within a few seconds.
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

/// Returned to the caller so window-event handlers can pause/resume the
/// listener based on visibility. `visible` is the source of truth read
/// from the loop; `thread` lets the handler `unpark()` after flipping it
/// back to `true`.
#[derive(Clone)]
pub struct ProbeHandle {
    pub visible: Arc<AtomicBool>,
    pub thread: Thread,
}

impl ProbeHandle {
    /// Flip the visibility flag and wake the listener if needed. Safe to
    /// call from any thread; idempotent under spurious window events.
    pub fn set_visible(&self, visible: bool) {
        let prev = self.visible.swap(visible, Ordering::Relaxed);
        if visible && !prev {
            self.thread.unpark();
        }
    }
}

pub fn spawn_listener(app: AppHandle) -> Option<ProbeHandle> {
    let visible = Arc::new(AtomicBool::new(true));
    let visible_for_thread = visible.clone();
    let handle = thread::Builder::new()
        .name("vrctext-vrc-listener".into())
        .spawn(move || run(app, visible_for_thread))
        .ok()?;
    Some(ProbeHandle {
        visible,
        thread: handle.thread().clone(),
    })
}

fn run(app: AppHandle, visible: Arc<AtomicBool>) {
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
    loop {
        // Park (0 CPU) when the user can't see the indicator. Loop on
        // wake to defend against spurious unparks and to re-check the
        // flag in case it flipped back to false during park.
        while !visible.load(Ordering::Relaxed) {
            thread::park();
        }
        match sock.recv_from(&mut buf) {
            Ok(_) => {
                let _ = app.emit("vrc-alive", ());
                // OS socket buffer (~256 KB) easily holds the ~60 Hz
                // burst arriving during this sleep; the next recv_from
                // returns immediately with whichever packet came first.
                thread::sleep(EMIT_THROTTLE);
            }
            Err(_) => continue, // RECV_TIMEOUT or transient — keep looping.
        }
    }
}
