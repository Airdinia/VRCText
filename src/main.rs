#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod download;
mod osc;
mod theme;
mod tts;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Lock DPI awareness to "system aware" BEFORE winit initializes its own
    // per-monitor awareness. The default PER_MONITOR_AWARE_V2 responds to
    // WM_DPICHANGED by scaling the window 1→1.5× when crossing a 100%→150%
    // monitor boundary mid-drag, which balloons the window so big you
    // literally can't finish the drag onto the higher-DPI screen. With
    // SYSTEM_AWARE, Windows bitmap-scales the window during cross-monitor
    // moves instead — slight blur on the secondary display, but size stays
    // constant and the drag completes smoothly. For a chat-sized window
    // that's the better trade.
    //
    // Must run before eframe::run_native, which calls winit's
    // `become_dpi_aware()` on first window creation. Once any value is set
    // via SetProcessDpiAwarenessContext, later calls return false — so
    // winit's attempt to switch to per-monitor silently no-ops.
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
        };
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 340.0])
            .with_min_inner_size([460.0, 240.0])
            .with_title("VRCText")
            .with_app_id("vrctext")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "VRCText",
        options,
        Box::new(|cc| Ok(Box::new(app::VRCTextApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    // Raw RGBA generated at build time (see build.rs); avoids pulling a PNG
    // decoder into the runtime binary.
    const RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.rgba"));
    egui::IconData {
        rgba: RGBA.to_vec(),
        width: env!("ICON_W").parse().unwrap(),
        height: env!("ICON_H").parse().unwrap(),
    }
}
