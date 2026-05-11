//! Windows-specific helpers shared across the backend.

#[cfg(windows)]
use std::net::UdpSocket;

/// Disable Windows' "UDP connection reset on ICMP unreachable" behavior so
/// `send_to` does not spuriously fail with `WSAECONNRESET` when VRChat is
/// closed (it sends ICMP port-unreachable, which Windows reports back to the
/// next UDP send on the same socket as a hard error).
#[cfg(windows)]
pub fn disable_udp_connreset(socket: &UdpSocket) {
    use std::os::windows::io::AsRawSocket;
    use windows::Win32::Networking::WinSock::{WSAIoctl, SIO_UDP_CONNRESET, SOCKET};

    let raw = socket.as_raw_socket() as usize;
    let sock = SOCKET(raw);
    let mut disable: u32 = 0;
    let mut bytes_returned: u32 = 0;
    unsafe {
        let _ = WSAIoctl(
            sock,
            SIO_UDP_CONNRESET,
            Some(&mut disable as *mut _ as *mut std::ffi::c_void),
            std::mem::size_of::<u32>() as u32,
            None,
            0,
            &mut bytes_returned,
            None,
            None,
        );
    }
}

#[cfg(not(windows))]
pub fn disable_udp_connreset(_socket: &std::net::UdpSocket) {}
