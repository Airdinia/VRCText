//! Tauri builder + state plumbing. Wires OSC, history, config,
//! TTS worker thread (SAPI + Sherpa), and the model downloader.

mod commands;
mod config;
mod download;
mod osc;
mod state;
mod tts;
mod tts_worker;
mod vrc_probe;
mod win;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Re-focus the existing window when a second launch is attempted.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }));
    }

    builder
        .setup(|app| {
            let cfg = config::Config::load();
            let initial_engine = cfg.engine;
            let initial_device = cfg.current_device().map(|s| s.to_string());
            let initial_voice = cfg.current_voice().map(|s| s.to_string());
            let always_on_top = cfg.always_on_top;
            drop(cfg);

            let tts = tts_worker::spawn(
                app.handle().clone(),
                initial_engine,
                initial_device,
                initial_voice,
            );
            let state = AppState::new(tts)?;

            // Apply persisted always-on-top before window first shows so the
            // user does not see a brief un-pinned flash.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(always_on_top);
                // Run with `VRCTEXT_DEVTOOLS=1` to open DevTools on launch.
                // Compiled out of release builds — pass `--features devtools`
                // when running `cargo tauri dev` to get the inspector.
                #[cfg(feature = "devtools")]
                if std::env::var("VRCTEXT_DEVTOOLS").as_deref() == Ok("1") {
                    window.open_devtools();
                }
            }
            app.manage(state);

            // Listen for VRChat's outbound OSC broadcasts on port 9001 so
            // the UI can show a green dot only when OSC is genuinely live,
            // not just because send_to() returned Ok (UDP can't tell).
            // Park the listener only when the window is minimised
            // — the user can't see the indicator anyway, no point spinning.
            // Losing focus (alt-tab) keeps the probe running so the status
            // stays up-to-date when the window regains focus.
            if let (Some(probe), Some(window)) = (
                vrc_probe::spawn_listener(app.handle().clone()),
                app.get_webview_window("main"),
            ) {
                let initial = !window.is_minimized().unwrap_or(false);
                probe.set_visible(initial);
                let app_handle = app.handle().clone();
                window.on_window_event(move |ev| {
                    use tauri::WindowEvent::Resized;
                    // Resized fires on minimise (size goes to 0×0 on
                    // Windows) and on restore. We only pause the probe
                    // when minimised — losing focus is fine.
                    if !matches!(ev, Resized(_)) {
                        return;
                    }
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let active = !w.is_minimized().unwrap_or(false);
                        probe.set_visible(active);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config_cmd::load_config,
            commands::config_cmd::save_partial_config,
            commands::osc_cmd::send_message,
            commands::osc_cmd::set_typing,
            commands::history_cmd::get_history,
            commands::history_cmd::clear_history,
            commands::history_cmd::resend_history,
            commands::window_cmd::set_always_on_top,
            commands::tts_cmd::tts_status,
            commands::tts_cmd::tts_speak,
            commands::tts_cmd::tts_stop,
            commands::tts_cmd::tts_set_enabled,
            commands::tts_cmd::tts_switch_engine,
            commands::tts_cmd::tts_list_devices,
            commands::tts_cmd::tts_list_voices,
            commands::tts_cmd::tts_apply_device,
            commands::tts_cmd::tts_apply_voice,
            commands::download_cmd::download_pack,
            commands::download_cmd::delete_models,
            commands::download_cmd::installed_packs,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            // The most common reason this fails on a fresh box is "WebView2
            // Runtime not installed" — Win11 has it preinstalled but old
            // Win10 systems sometimes don't. Pop a native MessageBox with the
            // download link so users don't see a silent crash.
            #[cfg(windows)]
            show_startup_error(&format!("{e}"));
            std::process::exit(1);
        });
}

#[cfg(windows)]
fn show_startup_error(msg: &str) {
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK,
    };

    // Fires before the frontend loads, so we can't read the user's language
    // preference — show both languages stacked instead.
    let body = format!(
        "VRCText 启动失败:\n\n{msg}\n\n\
         如果错误提到 WebView2,请安装 Microsoft WebView2 Runtime:\n\
         https://go.microsoft.com/fwlink/p/?LinkId=2124703\n\n\
         (Win11 默认已安装;部分老版本 Win10 需要手动安装)\n\n\
         ────────────────────────\n\n\
         VRCText failed to start:\n\n{msg}\n\n\
         If the error mentions WebView2, install the Microsoft WebView2 Runtime:\n\
         https://go.microsoft.com/fwlink/p/?LinkId=2124703\n\n\
         (Win11 ships with it; some older Win10 builds need to install it manually)"
    );
    let body_w: Vec<u16> = body.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        MessageBoxW(
            None,
            PCWSTR(body_w.as_ptr()),
            w!("VRCText 启动失败 / Startup failed"),
            MB_OK | MB_ICONERROR,
        );
    }
}
