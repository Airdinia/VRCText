// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Lock DPI awareness to "system aware" before wry/winit gets a chance to
    // request per-monitor-v2. The per-monitor default rescales the window
    // 1→1.5× when crossing a 100%→150% monitor boundary mid-drag, which
    // balloons the window so big you literally can't finish the drag.
    // SetProcessDpiAwarenessContext is one-shot — first writer wins — so this
    // call must happen before any window is created.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
    }

    vrctext_lib::run();
}
