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

/// Stamp the window's big (taskbar / Alt-Tab) and small (title bar) icons
/// from the icon embedded in the exe resources.
///
/// tao's `set_window_icon` only ever sends `ICON_SMALL`, so the taskbar's
/// big icon falls back to the shell's icon cache — which occasionally
/// misses and leaves a blank default icon after launch. An explicit
/// `WM_SETICON` from a live HICON bypasses the cache entirely.
#[cfg(windows)]
pub fn stamp_window_icon(hwnd: isize) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, LoadImageW, SendMessageW, ICON_BIG, ICON_SMALL, IMAGE_ICON,
        LR_DEFAULTCOLOR, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, WM_SETICON,
    };

    // tauri-build embeds `icons/icon.ico` at resource id 32512
    // (IDI_APPLICATION) so the exe doubles as its own icon source.
    const APP_ICON_ID: PCWSTR = PCWSTR(32512 as *const u16);

    unsafe {
        let Ok(module) = GetModuleHandleW(PCWSTR::null()) else {
            return;
        };
        let hinstance = HINSTANCE(module.0);
        let hwnd = HWND(hwnd as *mut _);
        for (kind, cx, cy) in [
            (ICON_BIG, SM_CXICON, SM_CYICON),
            (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
        ] {
            // LoadImageW picks the best-fitting layer out of the .ico
            // resource group for the requested metric, so the 16px small
            // icon stays crisp instead of being a scaled-down 32px one.
            if let Ok(icon) = LoadImageW(
                hinstance,
                APP_ICON_ID,
                IMAGE_ICON,
                GetSystemMetrics(cx),
                GetSystemMetrics(cy),
                LR_DEFAULTCOLOR,
            ) {
                SendMessageW(
                    hwnd,
                    WM_SETICON,
                    WPARAM(kind as usize),
                    LPARAM(icon.0 as isize),
                );
            }
        }
    }
}
