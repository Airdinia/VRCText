use crate::config::{Config, Engine};
use crate::download::{installed_kinds, ModelDownloader, ModelKind};
use crate::osc;
use crate::theme;
use crate::tts::{
    load_sherpa_async, Choice, LoadedSherpa, LoadingEngine, SapiEngine, SherpaEngine, TtsEngine,
};

use chrono::{DateTime, Datelike, Local, TimeZone};
use eframe::egui;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

const MAX_CHARS: usize = 144;
const TYPING_HEARTBEAT: Duration = Duration::from_millis(1500);
const TYPING_IDLE_OFF: Duration = Duration::from_millis(2000);
const STATUS_FADE: Duration = Duration::from_millis(1800);
const LONG_PRESS: Duration = Duration::from_millis(500);

// Shared layout constants so header / central panel / composer / cards all
// align on the same gutter lines. Changing one value here is enough to keep
// every surface visually consistent.
const PANEL_PAD_H: f32 = 14.0;
const ROW_PAD_H: f32 = 12.0;
const ROW_PAD_V: f32 = 7.0;
const ROW_INNER_H: f32 = 22.0;
const TIME_COL_W: f32 = 92.0;
const ACTION_COL_W: f32 = 28.0;
const COL_GAP: f32 = 8.0;
const LABEL_COL_W: f32 = 88.0;
const FIELD_COL_W: f32 = 220.0;

struct HoldState {
    index: usize,
    started: Instant,
    fired: bool,
}

enum RowAction {
    Append(usize),
    StartHold(usize),
}

pub struct VRCTextApp {
    text: String,
    config: Config,
    socket: UdpSocket,
    history_cursor: Option<usize>,
    typing_active: bool,
    last_keystroke: Instant,
    last_typing_sent: Instant,
    prev_text: String,
    show_settings: bool,
    ip_input: String,
    port_input: String,
    status: Option<(String, Instant)>,
    focus_requested: bool,
    applied_window_level: Option<bool>,
    hold: Option<HoldState>,
    clear_confirm_at: Option<Instant>,
    delete_models_confirm_at: Option<Instant>,
    tts: Box<dyn TtsEngine>,
    tts_devices: Vec<Choice>,
    tts_voices: Vec<Choice>,
    model_downloader: Option<ModelDownloader>,
    /// Active receiver when Sherpa is loading on a worker thread. Dropped
    /// as soon as the result is consumed (or when the user switches away
    /// from Sherpa and we no longer care about the outcome).
    sherpa_loader: Option<Receiver<Result<LoadedSherpa, String>>>,
}

impl VRCTextApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_cjk_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx);
        let config = Config::load();
        let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind UDP socket");
        let _ = socket.set_nonblocking(true);
        disable_udp_connreset(&socket);
        let now = Instant::now();
        let ip_input = config.ip.clone();
        let port_input = config.port.to_string();
        let (tts, sherpa_loader) = boot_engine(&config);
        let mut app = Self {
            text: String::new(),
            config,
            socket,
            history_cursor: None,
            typing_active: false,
            last_keystroke: now,
            last_typing_sent: now - TYPING_HEARTBEAT,
            prev_text: String::new(),
            show_settings: false,
            ip_input,
            port_input,
            status: None,
            focus_requested: false,
            applied_window_level: None,
            hold: None,
            clear_confirm_at: None,
            delete_models_confirm_at: None,
            tts,
            tts_devices: Vec::new(),
            tts_voices: Vec::new(),
            model_downloader: None,
            sherpa_loader,
        };
        // When a loader is running, the current `tts` is the `LoadingEngine`
        // stub — applying device/voice against it is a no-op and would
        // clobber saved settings with `None`. The async-load finisher does
        // the apply once the real engine is in place.
        if app.sherpa_loader.is_none() {
            app.apply_saved_audio_settings();
        }
        app
    }

    fn target(&self) -> Option<SocketAddr> {
        format!("{}:{}", self.config.ip, self.config.port).parse().ok()
    }

    fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    fn try_send_osc(&mut self, msg: &str) -> Result<(), &'static str> {
        let target = self.target().ok_or("目标地址无效")?;
        let pkt = osc::encode_chatbox_input(msg, true, self.config.play_sound);
        osc::send(&self.socket, target, &pkt).map_err(|_| "OSC 发送失败")?;
        if self.typing_active {
            let stop = osc::encode_chatbox_typing(false);
            let _ = osc::send(&self.socket, target, &stop);
            self.typing_active = false;
        }
        Ok(())
    }

    /// Unified send/resend pipeline: OSC → TTS → history.
    /// `ok_label` is the status text shown when OSC succeeds.
    fn dispatch(&mut self, msg: String, ok_label: &'static str) {
        let osc_result = self.try_send_osc(&msg);
        if self.config.tts_enabled {
            self.tts.speak(&msg);
        }
        self.config.push_history(msg);
        self.config.save();

        self.set_status(match (osc_result, self.config.tts_enabled) {
            (Ok(()), _) => ok_label,
            (Err(_), true) => "OSC 失败，仅朗读",
            (Err(e), false) => e,
        });
    }

    fn send_message(&mut self) {
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return;
        }
        let msg: String = trimmed.chars().take(MAX_CHARS).collect();
        self.text.clear();
        self.prev_text.clear();
        self.history_cursor = None;
        self.dispatch(msg, "已发送");
    }

    fn direct_resend(&mut self, index: usize) {
        let Some(entry) = self.config.history.get(index).cloned() else { return; };
        let msg: String = entry.text.chars().take(MAX_CHARS).collect();
        if msg.is_empty() {
            return;
        }
        self.dispatch(msg, "已重发");
    }

    fn append_to_input(&mut self, ctx: &egui::Context, index: usize) {
        let Some(entry) = self.config.history.get(index).cloned() else { return; };
        let insert = entry.text;
        let id = composer_edit_id();

        if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
            let range = state
                .cursor
                .char_range()
                .unwrap_or_else(|| {
                    let end = self.text.chars().count();
                    egui::text::CCursorRange::one(egui::text::CCursor::new(end))
                });
            let total = self.text.chars().count();
            let start = range.primary.index.min(range.secondary.index).min(total);
            let end = range.primary.index.max(range.secondary.index).min(total);

            let before: String = self.text.chars().take(start).collect();
            let after: String = self.text.chars().skip(end).collect();
            self.text = before + &insert + &after;

            let new_pos = start + insert.chars().count();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(new_pos),
            )));
            state.store(ctx, id);
        } else {
            self.text.push_str(&insert);
        }

        self.prev_text = self.text.clone();
        self.history_cursor = None;
        self.focus_requested = false;
    }

    fn send_typing(&mut self, typing: bool) {
        let Some(target) = self.target() else { return; };
        let pkt = osc::encode_chatbox_typing(typing);
        let _ = osc::send(&self.socket, target, &pkt);
        self.typing_active = typing;
        self.last_typing_sent = Instant::now();
    }

    fn set_status(&mut self, s: &str) {
        self.status = Some((s.to_string(), Instant::now()));
    }

    fn update_typing(&mut self) {
        let now = Instant::now();
        let text_changed = self.text != self.prev_text;
        if text_changed {
            self.last_keystroke = now;
            self.prev_text = self.text.clone();
            self.history_cursor = None;
            if !self.text.trim().is_empty()
                && (!self.typing_active
                    || now.duration_since(self.last_typing_sent) > TYPING_HEARTBEAT)
            {
                self.send_typing(true);
            }
        }
        if self.typing_active && now.duration_since(self.last_keystroke) > TYPING_IDLE_OFF {
            self.send_typing(false);
        }
    }

    fn history_prev(&mut self) {
        if self.config.history.is_empty() {
            return;
        }
        let new_idx = match self.history_cursor {
            None => self.config.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_cursor = Some(new_idx);
        if let Some(e) = self.config.history.get(new_idx) {
            self.text = e.text.clone();
        }
    }

    fn history_next(&mut self) {
        let Some(i) = self.history_cursor else { return; };
        if i + 1 >= self.config.history.len() {
            self.history_cursor = None;
            self.text.clear();
        } else {
            self.history_cursor = Some(i + 1);
            if let Some(e) = self.config.history.get(i + 1) {
                self.text = e.text.clone();
            }
        }
    }

    fn apply_window_level(&mut self, ctx: &egui::Context) {
        if self.applied_window_level == Some(self.config.always_on_top) {
            return;
        }
        let level = if self.config.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        self.applied_window_level = Some(self.config.always_on_top);
    }

    fn draw_history(&mut self, ui: &mut egui::Ui) {
        if self.config.history.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("暂无历史记录")
                        .size(14.0)
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("发送的消息会出现在这里")
                        .size(12.0)
                        .color(theme::TEXT_WEAK),
                );
            });
            return;
        }

        let now = Local::now();
        let hold_snapshot = self.hold.as_ref().map(|h| RowHold {
            index: h.index,
            fired: h.fired,
            progress: (h.started.elapsed().as_secs_f32() / LONG_PRESS.as_secs_f32())
                .clamp(0.0, 1.0),
        });

        let mut actions: Vec<RowAction> = Vec::new();
        for (idx, entry) in self.config.history.iter().enumerate() {
            let hold = match hold_snapshot {
                Some(ref h) if h.index == idx => Some(h),
                _ => None,
            };
            if let Some(a) = render_row(ui, idx, &entry.text, entry.ts, now, hold) {
                actions.push(a);
            }
            ui.add_space(2.0);
        }

        for a in actions {
            match a {
                RowAction::Append(i) => self.append_to_input(ui.ctx(), i),
                RowAction::StartHold(i) => {
                    self.hold = Some(HoldState {
                        index: i,
                        started: Instant::now(),
                        fired: false,
                    });
                }
            }
        }
    }

    fn update_hold(&mut self, ctx: &egui::Context) {
        let Some(hold) = &self.hold else { return; };
        let down = ctx.input(|i| i.pointer.primary_down());
        if !down {
            self.hold = None;
            return;
        }
        if !hold.fired && hold.started.elapsed() >= LONG_PRESS {
            let idx = hold.index;
            if let Some(h) = &mut self.hold {
                h.fired = true;
            }
            self.direct_resend(idx);
        }
    }
}

impl eframe::App for VRCTextApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.apply_window_level(ctx);
        self.poll_downloader(ctx);
        self.poll_sherpa_loader(ctx);

        // Enter = send, but never while the IME is actively composing —
        // otherwise pressing Enter to commit a pinyin candidate would
        // also fire off the half-typed message. The authoritative check
        // for "composition in progress" is the Win32 call
        // `ImmGetCompositionStringW(himc, GCS_COMPSTR, null, 0)`, which
        // returns the byte length of the current preedit string. This
        // is what every native Windows chat app (WeChat, QQ, Telegram
        // Desktop, ...) uses — neither winit's IME events nor egui's
        // text-buffer snapshots are reliable across MS Pinyin, TSF,
        // and WM_CHAR injection paths; IMM32 is the common denominator.
        //
        let ime_composing = is_ime_composing(frame);
        let enter_send = ctx.input_mut(|i| {
            let pressed = i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            pressed && !ime_composing
        });
        let hist_up = ctx.input_mut(|i| {
            self.text.is_empty()
                && i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
        });
        let hist_down = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                && self.history_cursor.is_some()
        });
        if hist_up {
            self.history_prev();
        }
        if hist_down {
            self.history_next();
        }

        self.draw_header(ctx);
        if !self.show_settings {
            self.draw_composer(ctx);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_BASE)
                    .inner_margin(egui::Margin::symmetric(PANEL_PAD_H, 10.0)),
            )
            .show(ctx, |ui| {
                if self.show_settings {
                    self.draw_settings_page(ui);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            self.draw_history(ui);
                        });
                }
            });

        if enter_send && !self.show_settings {
            self.send_message();
        }

        self.update_hold(ctx);

        // Typing indicator is driven by a separate UDP heartbeat and does not
        // need per-frame repaints; only fade-out status and hold progress do.
        let status_fading = self
            .status
            .as_ref()
            .map_or(false, |(_, at)| at.elapsed() < STATUS_FADE);
        if status_fading || self.hold.is_some() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.typing_active {
            self.send_typing(false);
        }
        self.tts.stop();
        self.config.save();
    }
}

impl VRCTextApp {
    fn draw_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("vrctext_header")
            .exact_height(48.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_SURFACE)
                    .inner_margin(egui::Margin::symmetric(PANEL_PAD_H, 0.0))
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        egui::RichText::new("VRCText")
                            .size(15.0)
                            .strong()
                            .color(theme::TEXT_PRIMARY),
                    );
                    ui.add_space(10.0);
                    status_chip(
                        ui,
                        theme::SUCCESS,
                        format!("OSC {}:{}", self.config.ip, self.config.port),
                    );

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            // Settings cogwheel
                            let gear_tip = if self.show_settings { "返回" } else { "设置" };
                            let gear = icon_button(ui, "⚙", self.show_settings, gear_tip);
                            if gear.clicked() {
                                self.ip_input = self.config.ip.clone();
                                self.port_input = self.config.port.to_string();
                                self.show_settings = !self.show_settings;
                                if self.show_settings {
                                    self.tts_devices = self.tts.device_choices();
                                    self.tts_voices = self.tts.voice_choices();
                                } else {
                                    self.tts_devices = Vec::new();
                                    self.tts_voices = Vec::new();
                                }
                            }

                            ui.add_space(6.0);
                            ui.add(
                                egui::Separator::default()
                                    .vertical()
                                    .spacing(0.0),
                            );
                            ui.add_space(6.0);

                            // TTS toggle — hidden entirely while the engine
                            // is unavailable (Sherpa still loading, or SAPI
                            // somehow dropped) so the two buttons to its
                            // left shift right to fill the gap instead of
                            // leaving a half-dead grayed-out slot. User can
                            // still manage engine state via the settings
                            // cogwheel next to it.
                            if self.tts.available() {
                                let tts_tip = match self.config.engine {
                                    Engine::Sapi => "语音朗读 (SAPI) · 关→开 可重载音频",
                                    Engine::Sherpa => "语音朗读 (AI) · 关→开 可重载音频",
                                };
                                let r = icon_button(ui, "🔊", self.config.tts_enabled, tts_tip);
                                if r.clicked() {
                                    self.config.tts_enabled = !self.config.tts_enabled;
                                    if !self.config.tts_enabled {
                                        self.tts.stop();
                                    } else {
                                        // Re-acquire OS audio handles in case
                                        // audiosrv was restarted while the
                                        // engine was idle — without this the
                                        // existing stream is silently dead.
                                        self.tts.reload();
                                        self.apply_saved_audio_settings();
                                        self.set_status("已重新加载语音输出");
                                    }
                                    self.config.save();
                                }
                            }

                            let r = icon_button(
                                ui,
                                "🔔",
                                self.config.play_sound,
                                "VRChat 消息提示音",
                            );
                            if r.clicked() {
                                self.config.play_sound = !self.config.play_sound;
                                self.config.save();
                            }

                            let r = icon_button(
                                ui,
                                "📌",
                                self.config.always_on_top,
                                "窗口置顶",
                            );
                            if r.clicked() {
                                self.config.always_on_top = !self.config.always_on_top;
                                self.config.save();
                            }
                        },
                    );
                });
            });
    }

    fn draw_composer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("vrctext_composer")
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(theme::BG_BASE)
                    .inner_margin(egui::Margin {
                        left: PANEL_PAD_H,
                        right: PANEL_PAD_H,
                        top: 8.0,
                        bottom: 8.0,
                    })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                // Framed multiline input. We deliberately avoid
                // `ui.add_sized(...)` here — forcing an outer rect on a
                // TextEdit stretches the hit-test region past the internal
                // text layout, and clicks that land in the stretched gap
                // get clamped to the end of the text instead of mapping to
                // the glyph under the cursor. Letting TextEdit size itself
                // via `desired_rows` keeps click → caret accurate.
                theme::input_frame().show(ui, |ui| {
                    let edit = egui::TextEdit::multiline(&mut self.text)
                        .id(composer_edit_id())
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .hint_text(
                            egui::RichText::new(
                                "输入消息  ·  [Enter] 发送  ·  [Shift+Enter] 换行",
                            )
                            .color(theme::TEXT_WEAK),
                        )
                        .frame(false);
                    let response = ui.add(edit);
                    if !self.focus_requested {
                        response.request_focus();
                        self.focus_requested = true;
                    }
                });

                self.update_typing();

                ui.add_space(4.0);

                // Slim footer row. All three elements (char count, status,
                // send button) share the same explicit text size so their
                // baselines match when the horizontal layout centers them
                // vertically — mismatched font sizes was why the count and
                // the button looked vertically offset from each other.
                const FOOTER_TEXT_SIZE: f32 = 12.0;
                const FOOTER_ROW_H: f32 = 22.0;

                ui.horizontal(|ui| {
                    ui.set_min_height(FOOTER_ROW_H);

                    // Character count
                    let count = self.char_count();
                    let color = if count > MAX_CHARS {
                        theme::DANGER
                    } else if count > MAX_CHARS - 20 {
                        theme::WARNING
                    } else {
                        theme::TEXT_WEAK
                    };
                    ui.label(
                        egui::RichText::new(format!("{} / {}", count, MAX_CHARS))
                            .size(FOOTER_TEXT_SIZE)
                            .color(color),
                    );

                    ui.add_space(10.0);

                    // Status pill — fades after STATUS_FADE
                    if let Some((msg, at)) = self.status.clone() {
                        if at.elapsed() < STATUS_FADE {
                            let alpha = 1.0
                                - (at.elapsed().as_secs_f32() / STATUS_FADE.as_secs_f32())
                                    .clamp(0.0, 1.0);
                            let c = theme::TEXT_SECONDARY
                                .gamma_multiply(alpha.max(0.4));
                            ui.label(
                                egui::RichText::new(msg)
                                    .size(FOOTER_TEXT_SIZE)
                                    .color(c),
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Shrink button_padding so the request
                        // min_size=(96, FOOTER_ROW_H) actually governs the
                        // button height — the default (12, 6) would push it
                        // to ~26px tall and leave the labels visually lower.
                        ui.spacing_mut().button_padding = egui::vec2(12.0, 3.0);
                        let btn = egui::Button::new(
                            egui::RichText::new("发送 [Enter]")
                                .size(FOOTER_TEXT_SIZE)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(theme::ACCENT)
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
                        .min_size(egui::vec2(96.0, FOOTER_ROW_H));
                        if ui.add(btn).clicked() {
                            self.send_message();
                        }
                    });
                });
            });
    }

    fn draw_settings_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(2.0);
                // ── Endpoint section ─────────────────────────────
                section_header(ui, "OSC 端点");
            theme::card_frame().show(ui, |ui| {
                egui::Grid::new("endpoint_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        // Don't wrap these in `ui.add_sized([w, h], ...)`.
                        // Forcing a taller-than-natural rect on a single-
                        // line TextEdit pins the text to the top of the
                        // box instead of centering it — the same pitfall
                        // that bit the composer's caret-on-click. Let the
                        // widget size itself vertically; width is fixed
                        // via `desired_width`.
                        grid_label(ui, "IP 地址");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.ip_input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(FIELD_COL_W),
                        );
                        ui.end_row();

                        grid_label(ui, "端口");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.port_input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(FIELD_COL_W),
                        );
                        ui.end_row();
                    });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    // Indent the action row so the "保存" button sits under the
                    // field column instead of floating off-axis from the
                    // labeled inputs above it.
                    ui.add_space(LABEL_COL_W + 10.0);
                    let save = theme::accent_button("保存", egui::vec2(72.0, 28.0));
                    if ui.add(save).clicked() {
                        let ip_ok = self.ip_input.parse::<std::net::IpAddr>().is_ok();
                        let port_ok = self.port_input.parse::<u16>().ok();
                        if ip_ok {
                            if let Some(p) = port_ok {
                                self.config.ip = self.ip_input.clone();
                                self.config.port = p;
                                self.config.save();
                                self.set_status("已保存");
                            } else {
                                self.set_status("端口无效");
                            }
                        } else {
                            self.set_status("IP 无效");
                        }
                    }
                    ui.label(
                        egui::RichText::new("默认 127.0.0.1 : 9000")
                            .size(11.0)
                            .color(theme::TEXT_WEAK),
                    );
                });
            });

            ui.add_space(12.0);

            // ── Voice section ────────────────────────────────
            section_header(ui, "语音朗读");
            theme::card_frame().show(ui, |ui| {
                // Engine selector stays visible even when the chosen backend
                // is unavailable — so the user can always swap to a working
                // one without needing to open the picker blind.
                let mut engine_pick: Option<Engine> = None;
                egui::Grid::new("engine_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        grid_label(ui, "引擎");
                        egui::ComboBox::from_id_salt("tts_engine_combo")
                            .selected_text(engine_label(self.config.engine))
                            .width(FIELD_COL_W)
                            .show_ui(ui, |ui| {
                                for e in [Engine::Sapi, Engine::Sherpa] {
                                    let sel = self.config.engine == e;
                                    if ui
                                        .selectable_label(sel, engine_label(e))
                                        .clicked()
                                        && !sel
                                    {
                                        engine_pick = Some(e);
                                    }
                                }
                            });
                        ui.end_row();
                    });
                if let Some(e) = engine_pick {
                    self.switch_engine(e);
                }
                ui.add_space(6.0);

                if !self.tts.available() {
                    match self.config.engine {
                        Engine::Sapi => {
                            ui.label(
                                egui::RichText::new(engine_unavailable_msg(Engine::Sapi))
                                    .color(theme::DANGER),
                            );
                        }
                        Engine::Sherpa => self.render_sherpa_unavailable(ui),
                    }
                } else {
                    let mut device_pick: Option<Option<String>> = None;
                    let mut voice_pick: Option<Option<String>> = None;
                    egui::Grid::new("voice_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            grid_label(ui, "输出设备");
                            device_pick = choice_combo(
                                ui,
                                "tts_device_combo",
                                self.config.current_device(),
                                &self.tts_devices,
                            );
                            ui.end_row();

                            grid_label(ui, "语音 / 语言");
                            voice_pick = choice_combo(
                                ui,
                                "tts_voice_combo",
                                self.config.current_voice(),
                                &self.tts_voices,
                            );
                            ui.end_row();
                        });

                    if let Some(new_key) = device_pick {
                        let applied = self.tts.apply_device(new_key.as_deref());
                        self.config.set_current_device(applied);
                        self.config.save();
                        self.set_status("TTS 输出设备已切换");
                    }
                    if let Some(new_key) = voice_pick {
                        let applied = self.tts.apply_voice(new_key.as_deref());
                        self.config.set_current_voice(applied);
                        self.config.save();
                        self.set_status("TTS 语音已切换");
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(LABEL_COL_W + 10.0);
                        if ui
                            .button("🔈 试听")
                            .on_hover_text("用当前选中语音+输出设备念一句测试")
                            .clicked()
                        {
                            self.tts.speak("你好 hello，这是测试语音");
                        }
                        if ui
                            .button("刷新设备")
                            .on_hover_text("重新扫描音频设备和语音")
                            .clicked()
                        {
                            self.tts_devices = self.tts.device_choices();
                            self.tts_voices = self.tts.voice_choices();
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "💡 要路由到 VRChat 麦克风需配合虚拟音频线缆\
                             （如 CABLE Input）作为输出设备。",
                        )
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                    );
                }

                // Let users add a second pack (e.g. Kokoro on top of Matcha)
                // without having to first delete what they have. Shown only
                // when the engine is up and at least one pack is still
                // missing. During an active download we hand over to the
                // progress UI rendered in `render_sherpa_unavailable` path.
                if self.config.engine == Engine::Sherpa
                    && self.tts.available()
                    && self.model_downloader.is_none()
                {
                    let missing = match crate::config::models_dir() {
                        Some(dir) => {
                            let installed = installed_kinds(&dir);
                            [ModelKind::MatchaZhBaker, ModelKind::KokoroMultiLang]
                                .into_iter()
                                .any(|k| !installed.contains(&k))
                        }
                        None => false,
                    };
                    if missing {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("添加更多语音包")
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                        );
                        ui.add_space(4.0);
                        self.render_download_buttons(ui);
                    }
                }

                // While a download is running *and* the engine is already
                // up (Matcha present, user added Kokoro on top), show
                // progress inline. When the engine isn't available yet the
                // !available branch above already rendered the same
                // progress UI — so we must not render it a second time.
                if self.tts.available() && self.model_downloader.is_some() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                    self.render_sherpa_unavailable(ui);
                }

                // Delete-downloaded-models lives inside the AI card regardless
                // of availability so users can nuke a half-downloaded/corrupt
                // bundle that's keeping the engine from loading.
                if self.config.engine == Engine::Sherpa
                    && self.model_downloader.is_none()
                    && models_present()
                {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                    self.render_delete_models_row(ui);
                }
            });

            ui.add_space(12.0);

            // ── History section ──────────────────────────────
            section_header(ui, "历史记录");
            theme::card_frame().show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "当前 {} 条  ·  上限 {} 条（超出自动丢弃最早）",
                        self.config.history.len(),
                        crate::config::HISTORY_CAP
                    ))
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
                );
                ui.add_space(6.0);

                let count = self.config.history.len();
                let confirming = self
                    .clear_confirm_at
                    .map_or(false, |t| t.elapsed() < Duration::from_secs(5));
                if !confirming {
                    ui.add_enabled_ui(count > 0, |ui| {
                        let btn = egui::Button::new(
                            egui::RichText::new("🗑  清空全部历史").color(theme::DANGER),
                        )
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD));
                        if ui.add(btn).clicked() {
                            self.clear_confirm_at = Some(Instant::now());
                        }
                    });
                } else {
                    ui.colored_label(
                        theme::DANGER,
                        format!("⚠ 确认清空 {} 条历史？此操作不可撤销", count),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let confirm = egui::Button::new(
                            egui::RichText::new("确认清空")
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .fill(theme::DANGER)
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD));
                        if ui.add(confirm).clicked() {
                            self.config.history.clear();
                            self.history_cursor = None;
                            self.hold = None;
                            self.config.save();
                            self.clear_confirm_at = None;
                            self.set_status(&format!("已清空 {} 条", count));
                        }
                        if ui.button("取消").clicked() {
                            self.clear_confirm_at = None;
                        }
                    });
                }
            });

                ui.add_space(8.0);
            });
    }

    /// Apply the user's saved device + voice to whatever engine is currently
    /// live, persisting any fallback the engine chose (e.g. when the saved
    /// device has been unplugged since last run).
    fn apply_saved_audio_settings(&mut self) {
        let mut cfg_dirty = false;
        let resolved_device = self.tts.apply_device(self.config.current_device());
        if resolved_device.as_deref() != self.config.current_device() {
            self.config.set_current_device(resolved_device);
            cfg_dirty = true;
        }
        let resolved_voice = self.tts.apply_voice(self.config.current_voice());
        if resolved_voice.as_deref() != self.config.current_voice() {
            self.config.set_current_voice(resolved_voice);
            cfg_dirty = true;
        }
        if cfg_dirty {
            self.config.save();
        }
    }

    /// Drain the Sherpa loader channel once per frame. When the worker
    /// returns a model, promote it to a real `SherpaEngine` on the UI
    /// thread (cpal streams must be built here, not on the worker).
    fn poll_sherpa_loader(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.sherpa_loader else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(loaded)) => {
                let engine = SherpaEngine::from_loaded(loaded, self.config.current_device());
                self.tts = Box::new(engine);
                self.sherpa_loader = None;
                self.tts_devices = self.tts.device_choices();
                self.tts_voices = self.tts.voice_choices();
                self.apply_saved_audio_settings();
                self.set_status("AI 引擎已就绪");
            }
            Ok(Err(_msg)) => {
                // Fall back to a synchronous construction so the engine's
                // own init_error surfaces the real failure state to the UI.
                self.tts = Box::new(SherpaEngine::new());
                self.sherpa_loader = None;
                self.tts_devices = self.tts.device_choices();
                self.tts_voices = self.tts.voice_choices();
            }
            Err(TryRecvError::Empty) => {
                // Keep repainting so the user sees the loading state even
                // while they're not interacting with the window.
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            Err(TryRecvError::Disconnected) => {
                self.sherpa_loader = None;
            }
        }
    }

    /// Erase everything under `%APPDATA%\vrctext\models\` and rebuild the
    /// current engine. If config wanted Sherpa, it comes back unavailable
    /// and the download button reappears — which is what the user asked for.
    fn delete_downloaded_models(&mut self) {
        if self.model_downloader.is_some() {
            self.set_status("请先等待下载完成或取消");
            return;
        }
        let Some(dir) = crate::config::models_dir() else {
            self.set_status("无法解析模型目录");
            return;
        };
        // Release the current engine's grip on the model before we touch
        // the filesystem; OfflineTts doesn't keep files open after `create`,
        // but swapping to SAPI drops the Arc deterministically so any
        // in-flight synth thread's callback sees a cancelled counter.
        self.tts.stop();
        self.tts = Box::new(SapiEngine::new());
        // Any in-flight Sherpa load is about to be looking at files we're
        // deleting — drop its channel so we ignore whatever comes back.
        self.sherpa_loader = None;
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => self.set_status("已删除本地 AI 模型"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                self.set_status(&format!("删除失败: {e}"));
                // Still try to rebuild below — the user's intent was to
                // clear the AI state even if a stray file blocked us.
            }
        }
        // Models are gone now, so boot_engine returns the synchronous
        // "未检测到 AI 模型" state (no loader spawned).
        let (tts, _loader) = boot_engine(&self.config);
        self.tts = tts;
        self.tts_devices = self.tts.device_choices();
        self.tts_voices = self.tts.voice_choices();
    }

    /// Kick off a background download of the given pack into
    /// `%APPDATA%\vrctext\models\`. The worker thread updates its
    /// `DownloadState` in place; `poll_downloader` observes progress each
    /// frame and triggers engine reload on completion.
    fn start_model_download(&mut self, kind: ModelKind) {
        if self.model_downloader.is_some() {
            return;
        }
        match crate::config::models_dir() {
            Some(dir) => {
                self.model_downloader = Some(ModelDownloader::start(kind, dir));
                self.set_status(&format!("开始下载 {}…", kind.display_name()));
            }
            None => self.set_status("无法解析模型目录"),
        }
    }

    /// While a download is running, keep repainting so the progress bar moves
    /// even without user input. On success, rebuild the engine so the newly
    /// downloaded model becomes selectable.
    fn poll_downloader(&mut self, ctx: &egui::Context) {
        let Some(dl) = &self.model_downloader else {
            return;
        };
        let state = dl.snapshot();
        if !state.done {
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if state.error.is_some() {
            // Leave the downloader in place so the UI can render the error
            // and expose a retry button. Cleared only by user action.
            return;
        }
        self.model_downloader = None;
        let (tts, loader) = boot_engine(&self.config);
        self.tts = tts;
        self.sherpa_loader = loader;
        self.tts_voices = self.tts.voice_choices();
        self.tts_devices = self.tts.device_choices();
        // If the Sherpa path is async (normal case after a successful
        // download), `poll_sherpa_loader` applies saved device/voice once
        // the worker thread returns. Only touch settings here for the
        // synchronous branches (SAPI or the rare Sherpa-no-models case).
        if self.sherpa_loader.is_none() {
            self.apply_saved_audio_settings();
        }
        self.set_status("AI 模型已就绪");
    }

    fn render_delete_models_row(&mut self, ui: &mut egui::Ui) {
        let confirming = self
            .delete_models_confirm_at
            .map_or(false, |t| t.elapsed() < Duration::from_secs(5));
        if !confirming {
            let btn = egui::Button::new(
                egui::RichText::new("🗑  删除已下载模型").color(theme::DANGER),
            );
            if ui
                .add(btn)
                .on_hover_text("移除 %APPDATA%\\vrctext\\models\\ 下的全部文件")
                .clicked()
            {
                self.delete_models_confirm_at = Some(Instant::now());
            }
        } else {
            ui.colored_label(theme::DANGER, "⚠ 确认删除本地 AI 模型？下次使用需重新下载");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let confirm = egui::Button::new(
                    egui::RichText::new("确认删除")
                        .color(egui::Color32::WHITE)
                        .strong(),
                )
                .fill(theme::DANGER);
                if ui.add(confirm).clicked() {
                    self.delete_downloaded_models();
                    self.delete_models_confirm_at = None;
                }
                if ui.button("取消").clicked() {
                    self.delete_models_confirm_at = None;
                }
            });
        }
    }

    fn render_sherpa_unavailable(&mut self, ui: &mut egui::Ui) {
        let snapshot = self.model_downloader.as_ref().map(|d| (d.kind, d.snapshot()));
        match snapshot {
            None => {
                // Three modes land here, distinguished below:
                //   1. a background loader is mid-flight → "加载中…" spinner
                //      (LoadingEngine returns detail text but it's NOT a
                //      failure; painting it red would be misleading UX)
                //   2. files exist but the engine failed to start → red error
                //   3. no files at all → neutral prompt + download buttons
                if self.sherpa_loader.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("AI 引擎加载中…")
                                .color(theme::TEXT_SECONDARY),
                        );
                    });
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(
                            "模型正在后台加载，完成后会自动切换。",
                        )
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                    );
                } else {
                    match self.tts.unavailable_detail() {
                        Some(detail) => {
                            ui.label(
                                egui::RichText::new("AI 引擎启动失败")
                                    .color(theme::DANGER),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(detail)
                                    .size(11.0)
                                    .color(theme::TEXT_WEAK),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "可尝试点下方\"删除已下载模型\"然后重新下载。",
                                )
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                            );
                        }
                        None => {
                            ui.label(
                                egui::RichText::new("未检测到 AI 模型")
                                    .color(theme::WARNING),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "下载一个语音模型包到 %APPDATA%\\vrctext\\models\\。\
                                     Matcha 体积小但仅中文；Kokoro 大得多，支持中英混读。",
                                )
                                .size(11.0)
                                .color(theme::TEXT_WEAK),
                            );
                            ui.add_space(6.0);
                            self.render_download_buttons(ui);
                        }
                    }
                }
            }
            Some((kind, state)) => {
                ui.label(format!("{} — {}", kind.display_name(), state.status));
                ui.add(
                    egui::ProgressBar::new(state.progress)
                        .show_percentage()
                        .desired_width(260.0),
                );
                if let Some(err) = &state.error {
                    ui.add_space(4.0);
                    ui.colored_label(theme::DANGER, format!("下载失败: {err}"));
                    ui.horizontal(|ui| {
                        if ui.button("重试").clicked() {
                            self.model_downloader = None;
                            self.start_model_download(kind);
                        }
                        if ui.button("取消").clicked() {
                            self.model_downloader = None;
                        }
                    });
                }
            }
        }
    }

    /// Offer buttons for every pack that isn't installed yet. Called from
    /// both the "no models" unavailable path and the available-card extras
    /// row (so users can add Kokoro after Matcha was already downloaded).
    fn render_download_buttons(&mut self, ui: &mut egui::Ui) {
        let installed = match crate::config::models_dir() {
            Some(dir) => installed_kinds(&dir),
            None => Vec::new(),
        };
        let mut pick: Option<ModelKind> = None;
        ui.horizontal_wrapped(|ui| {
            for kind in [ModelKind::MatchaZhBaker, ModelKind::KokoroMultiLang] {
                if installed.contains(&kind) {
                    continue;
                }
                let label = format!("下载 {} ({})", kind.display_name(), kind.size_hint());
                if ui.button(label).clicked() {
                    pick = Some(kind);
                }
            }
        });
        if let Some(kind) = pick {
            self.start_model_download(kind);
        }
    }

    /// Swap TTS backend and refresh the device/voice picker state. Each
    /// engine has its own slot in `Config` for device/voice names, so
    /// switching engines never clobbers the other engine's remembered
    /// selection.
    fn switch_engine(&mut self, next: Engine) {
        if self.config.engine == next {
            return;
        }
        self.tts.stop();
        self.config.engine = next;
        // `boot_engine` may hand back a LoadingEngine + async loader; for
        // SAPI it's cheap and synchronous. In the async case the loader
        // tick handles apply_device/apply_voice once the real engine is
        // ready, so skip it here to avoid overwriting config with None.
        let (tts, loader) = boot_engine(&self.config);
        self.tts = tts;
        self.sherpa_loader = loader;
        self.tts_devices = self.tts.device_choices();
        self.tts_voices = self.tts.voice_choices();
        if self.sherpa_loader.is_none() {
            let _ = self.tts.apply_device(self.config.current_device());
            let _ = self.tts.apply_voice(self.config.current_voice());
        }
        self.config.save();
        self.set_status(match next {
            Engine::Sapi => "已切换到 SAPI",
            Engine::Sherpa => "已切换到 AI 引擎",
        });
    }
}

/// Does `%APPDATA%\vrctext\models\` contain anything? Used to gate the
/// "删除已下载模型" button — we hide it when there's nothing to delete.
fn models_present() -> bool {
    match crate::config::models_dir() {
        Some(dir) => std::fs::read_dir(&dir)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false),
        None => false,
    }
}

/// Construct the engine the config asks for, moving any slow work to a
/// worker thread. SAPI is cheap to build so it returns fully-formed; the
/// Sherpa path returns a `LoadingEngine` placeholder plus a channel the
/// UI polls each frame to swap in the real model once ONNX finishes
/// loading. Callers that don't want async should use `build_engine`.
fn boot_engine(config: &Config) -> (Box<dyn TtsEngine>, Option<Receiver<Result<LoadedSherpa, String>>>) {
    match config.engine {
        Engine::Sapi => (Box::new(SapiEngine::new()), None),
        Engine::Sherpa => {
            // If models aren't installed we want the "未检测到 AI 模型" +
            // download-button UI to appear immediately, not a spinner that
            // will never resolve. Defer the background loader until we know
            // there's actually something to load.
            if models_present() {
                let preferred = config.current_voice().map(|s| s.to_string());
                (Box::new(LoadingEngine), Some(load_sherpa_async(preferred)))
            } else {
                (Box::new(SherpaEngine::new()), None)
            }
        }
    }
}

fn engine_label(e: Engine) -> &'static str {
    match e {
        Engine::Sapi => "SAPI（系统默认）",
        Engine::Sherpa => "AI 引擎（Matcha / Kokoro）",
    }
}

fn engine_unavailable_msg(e: Engine) -> &'static str {
    match e {
        Engine::Sapi => "系统 SAPI 不可用",
        Engine::Sherpa => "AI 引擎未就绪（未下载模型或加载失败）",
    }
}

fn composer_edit_id() -> egui::Id {
    egui::Id::new("vrctext_composer_edit")
}

fn status_chip(ui: &mut egui::Ui, dot: egui::Color32, text: impl Into<String>) {
    let txt: String = text.into();
    let frame = egui::Frame::none()
        .fill(theme::BG_ELEVATED)
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
        .stroke(egui::Stroke::new(1.0, theme::BORDER));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.5, dot);
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(txt)
                    .size(11.0)
                    .color(theme::TEXT_SECONDARY),
            );
        });
    });
}

fn icon_button(
    ui: &mut egui::Ui,
    glyph: &str,
    selected: bool,
    tooltip: &str,
) -> egui::Response {
    let fill = if selected {
        theme::ACCENT.linear_multiply(0.25)
    } else {
        egui::Color32::TRANSPARENT
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, theme::ACCENT)
    } else {
        egui::Stroke::new(1.0, theme::BORDER)
    };
    let btn = egui::Button::new(egui::RichText::new(glyph).size(15.0))
        .fill(fill)
        .stroke(stroke)
        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
        .min_size(egui::vec2(30.0, 28.0));
    ui.add(btn).on_hover_text(tooltip)
}

fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(11.0)
            .strong()
            .color(theme::TEXT_WEAK),
    );
    ui.add_space(4.0);
}

fn label_text(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(12.0)
        .color(theme::TEXT_SECONDARY)
}

/// Render a settings-grid label at a fixed column width so every grid on
/// the page lines up on the same vertical axis — the endpoint, engine, and
/// voice grids would otherwise compute their first-column widths
/// independently and end up visibly staggered.
fn grid_label(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [LABEL_COL_W, ROW_INNER_H],
        egui::Label::new(label_text(text)),
    )
}

/// Render a single-select ComboBox over `Choice` items. Returns the picked
/// key wrapped in `Some(Option<String>)` when the user changed the selection
/// (inner `None` means "系统默认"); returns `None` when nothing changed.
fn choice_combo(
    ui: &mut egui::Ui,
    id: &str,
    current_key: Option<&str>,
    items: &[Choice],
) -> Option<Option<String>> {
    let selected_label = items
        .iter()
        .find(|c| c.key.as_deref() == current_key)
        .map(|c| c.label.as_str())
        .unwrap_or("—");
    let mut picked: Option<Option<String>> = None;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected_label)
            .width(FIELD_COL_W)
            .show_ui(ui, |ui| {
                for choice in items {
                    let is_selected = choice.key.as_deref() == current_key;
                    if ui
                        .selectable_label(is_selected, &choice.label)
                        .clicked()
                        && !is_selected
                    {
                        picked = Some(choice.key.clone());
                    }
                }
            });
    });
    picked
}

struct RowHold {
    index: usize,
    fired: bool,
    progress: f32,
}

fn render_row(
    ui: &mut egui::Ui,
    idx: usize,
    text: &str,
    ts: i64,
    now: DateTime<Local>,
    hold: Option<&RowHold>,
) -> Option<RowAction> {
    let is_active_hold = hold.is_some();
    let hold_fired = hold.map(|h| h.fired).unwrap_or(false);
    let progress = hold.map(|h| h.progress).unwrap_or(0.0);

    let row_id = egui::Id::new(("vrctext_row_hover", idx));
    let prev_hover = ui
        .ctx()
        .memory(|m| m.data.get_temp::<bool>(row_id))
        .unwrap_or(false);
    let highlighted = prev_hover || is_active_hold;

    let mut action: Option<RowAction> = None;

    // Transparent rows by default — only the hovered row gets a fill, so
    // the list reads as plain text over the panel rather than a stack of
    // colored cards. The frame's inner_margin still reserves space for the
    // hover highlight without shifting the body text when the pointer moves.
    let row_fill = if highlighted {
        theme::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };

    let frame = egui::Frame::none()
        .fill(row_fill)
        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
        .inner_margin(egui::Margin::symmetric(ROW_PAD_H, ROW_PAD_V));

    let row_resp = frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Lock every child to the same inner height so the timestamp,
                // body text, and action button share a single vertical axis.
                ui.set_min_height(ROW_INNER_H);

                // Timestamp column — fixed width so text columns line up
                // between rows regardless of "刚刚" vs "2024-01-15 10:30".
                ui.add_sized(
                    [TIME_COL_W, ROW_INNER_H],
                    egui::Label::new(
                        egui::RichText::new(format_timestamp(ts, now))
                            .size(11.0)
                            .color(theme::TEXT_WEAK),
                    ),
                );

                ui.add_space(COL_GAP);

                // Text column — flex, leaves a fixed slot for the action
                // button on the right. Computed width is constant whether
                // the row is hovered or not, so the body glyphs never shift.
                let text_w = (ui.available_width() - ACTION_COL_W - COL_GAP).max(40.0);
                let text_label = egui::Label::new(
                    egui::RichText::new(text)
                        .size(13.0)
                        .color(theme::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense::hover());
                let text_resp = ui.add_sized([text_w, ROW_INNER_H], text_label);
                if text.chars().count() > 30 {
                    text_resp.on_hover_text(text);
                }

                ui.add_space(COL_GAP);

                // Resend button slot — reserved at fixed width even when
                // not hovered, so hovering doesn't reflow the row.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if highlighted {
                        // Default button_padding (12, 6) would make the
                        // button ~26px tall (text 14 + 12 padding), pushing
                        // this hovered row 4px taller than its neighbors.
                        // The list then jitters as the pointer moves between
                        // rows. Shrink the padding so the button fits inside
                        // ROW_INNER_H exactly and hover is geometry-neutral.
                        ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);

                        let btn_fill = if is_active_hold {
                            if hold_fired {
                                theme::SUCCESS.linear_multiply(0.25)
                            } else {
                                theme::WARNING.linear_multiply(0.25)
                            }
                        } else {
                            theme::BG_ELEVATED
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new("↺").size(13.0).color(theme::TEXT_PRIMARY),
                        )
                        .fill(btn_fill)
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER_STRONG))
                        .min_size(egui::vec2(ACTION_COL_W, ROW_INNER_H));
                        let resp = ui.add(btn).on_hover_text(
                            "短按：添加到输入框\n长按 0.5 秒：直接重发",
                        );
                        if resp.clicked() && !hold_fired {
                            action = Some(RowAction::Append(idx));
                        }
                        if resp.is_pointer_button_down_on() && !is_active_hold {
                            action = Some(RowAction::StartHold(idx));
                        }
                        if is_active_hold {
                            let btn_rect = resp.rect;
                            let bar_h = 2.5;
                            let bar_rect = egui::Rect::from_min_size(
                                egui::pos2(btn_rect.min.x, btn_rect.max.y + 2.0),
                                egui::vec2(btn_rect.width() * progress, bar_h),
                            );
                            let fill = if hold_fired {
                                theme::SUCCESS
                            } else {
                                theme::WARNING
                            };
                            ui.painter().rect_filled(
                                bar_rect,
                                egui::Rounding::same(1.0),
                                fill,
                            );
                        }
                    } else {
                        ui.add_space(ACTION_COL_W);
                    }
                });
            });
        })
        .response;

    let now_hover = row_resp.contains_pointer();
    if now_hover != prev_hover {
        ui.ctx()
            .memory_mut(|m| m.data.insert_temp(row_id, now_hover));
        ui.ctx().request_repaint();
    }

    action
}

fn format_timestamp(ts: i64, now: DateTime<Local>) -> String {
    if ts <= 0 {
        return "—".into();
    }
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else { return "—".into(); };
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();
    if (-60..60).contains(&secs) {
        return "刚刚".into();
    }
    let mins = diff.num_minutes();
    if (0..60).contains(&mins) {
        return format!("{} 分钟前", mins);
    }
    let today = now.date_naive();
    let dt_date = dt.date_naive();
    if dt_date == today {
        return format!("今天 {}", dt.format("%H:%M"));
    }
    let yesterday = today.pred_opt();
    if yesterday == Some(dt_date) {
        return format!("昨天 {}", dt.format("%H:%M"));
    }
    if dt.year() == now.year() {
        return dt.format("%m-%d %H:%M").to_string();
    }
    dt.format("%Y-%m-%d %H:%M").to_string()
}

fn setup_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyh.ttf",
        r"C:\Windows\Fonts\msyhl.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
    ];
    for path in candidates {
        // mmap the font instead of reading it into an owned Vec<u8>. The file
        // is mapped read-only and leaked so the &'static slice hands off to
        // epaint's Cow<'static, [u8]>; pages are file-backed, so Windows can
        // evict them under memory pressure and they don't count against
        // private commit like `fs::read` would.
        let Ok(file) = std::fs::File::open(path) else { continue };
        let Ok(mmap) = (unsafe { memmap2::Mmap::map(&file) }) else { continue };
        let leaked: &'static memmap2::Mmap = Box::leak(Box::new(mmap));
        fonts
            .font_data
            .insert("cjk".into(), egui::FontData::from_static(&leaked[..]));
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, "cjk".into());
        }
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.push("cjk".into());
        }
        break;
    }
    ctx.set_fonts(fonts);
}

/// Disable Windows' "UDP connection reset on ICMP unreachable" behavior so
/// `send_to` does not spuriously fail with WSAECONNRESET when VRChat is closed.
fn disable_udp_connreset(socket: &UdpSocket) {
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

/// Ask Windows IMM32 whether the current input context has an active
/// composition string (i.e. the IME is holding pre-commit characters).
/// Any non-zero `GCS_COMPSTR` length means the user is mid-composition and
/// Enter belongs to the IME. Returns false if we can't resolve an HWND or
/// the IMM calls fail — fall through to normal send behavior in that case.
fn is_ime_composing(frame: &eframe::Frame) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Input::Ime::{
        ImmGetCompositionStringW, ImmGetContext, ImmReleaseContext, GCS_COMPSTR,
    };

    let Ok(wh) = frame.window_handle() else { return false; };
    let hwnd = match wh.as_raw() {
        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
        _ => return false,
    };
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.0.is_null() {
            return false;
        }
        // Passing a null buffer with size 0 returns the required byte count
        // of the composition string (or 0 when there's no composition).
        let size = ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0);
        let _ = ImmReleaseContext(hwnd, himc);
        size > 0
    }
}
