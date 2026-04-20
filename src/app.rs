use crate::config::{self, Config, HistoryEntry};
use crate::osc;
use crate::theme;
use crate::tts::{
    TtsDevice, TtsEngine, DEFAULT_DEVICE_LABEL, DEFAULT_VOICE_LABEL, DEVICE_DEFAULT,
};

use eframe::egui;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const MAX_CHARS: usize = 144;
const TYPING_HEARTBEAT: Duration = Duration::from_millis(1500);
const TYPING_IDLE_OFF: Duration = Duration::from_millis(2000);
const STATUS_FADE: Duration = Duration::from_millis(1800);
const LONG_PRESS: Duration = Duration::from_millis(2000);

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
    tts: TtsEngine,
    tts_devices: Vec<TtsDevice>,
    tts_voices: Vec<String>,
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
            tts: TtsEngine::new(),
            tts_devices: Vec::new(),
            tts_voices: Vec::new(),
        };
        let mut cfg_dirty = false;
        let resolved_device = app
            .tts
            .apply_device_by_name(app.config.tts_device_name.as_deref());
        if resolved_device.as_deref() != app.config.tts_device_name.as_deref() {
            app.config.tts_device_name = resolved_device;
            cfg_dirty = true;
        }
        let resolved_voice = app
            .tts
            .apply_voice_by_name(app.config.tts_voice_name.as_deref());
        if resolved_voice.as_deref() != app.config.tts_voice_name.as_deref() {
            app.config.tts_voice_name = resolved_voice;
            cfg_dirty = true;
        }
        if cfg_dirty {
            app.config.save();
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

    fn send_message(&mut self) {
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return;
        }
        let msg: String = trimmed.chars().take(MAX_CHARS).collect();

        let osc_result = self.try_send_osc(&msg);
        if self.config.tts_enabled {
            self.tts.speak(&msg);
        }
        self.config.push_history(msg);
        self.config.save();
        self.text.clear();
        self.prev_text.clear();
        self.history_cursor = None;

        self.set_status(match (osc_result, self.config.tts_enabled) {
            (Ok(()), _) => "已发送",
            (Err(e), true) => {
                let _ = e;
                "OSC 失败，仅朗读"
            }
            (Err(e), false) => e,
        });
    }

    fn direct_resend(&mut self, index: usize) {
        let Some(entry) = self.config.history.get(index).cloned() else { return; };
        let msg: String = entry.text.chars().take(MAX_CHARS).collect();
        if msg.is_empty() {
            return;
        }

        let osc_result = self.try_send_osc(&msg);
        if self.config.tts_enabled {
            self.tts.speak(&msg);
        }
        self.config.push_history(msg);
        self.config.save();

        self.set_status(match (osc_result, self.config.tts_enabled) {
            (Ok(()), _) => "已重发",
            (Err(_), true) => "OSC 失败，仅朗读",
            (Err(e), false) => e,
        });
    }

    fn append_to_input(&mut self, index: usize) {
        let Some(entry) = self.config.history.get(index) else { return; };
        if self.text.is_empty() {
            self.text = entry.text.clone();
        } else {
            if !self.text.ends_with(' ') && !self.text.ends_with('\n') {
                self.text.push(' ');
            }
            self.text.push_str(&entry.text);
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
        let entries: Vec<(String, i64)> = self
            .config
            .history
            .iter()
            .map(|e| (e.text.clone(), e.ts))
            .collect();

        if entries.is_empty() {
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

        let active_hold_idx = self.hold.as_ref().map(|h| h.index);
        let hold_progress = self.hold.as_ref().map(|h| {
            (h.started.elapsed().as_secs_f32() / LONG_PRESS.as_secs_f32()).clamp(0.0, 1.0)
        });
        let hold_fired = self.hold.as_ref().map(|h| h.fired).unwrap_or(false);

        let mut actions: Vec<RowAction> = Vec::new();

        for (idx, (text, ts)) in entries.iter().enumerate() {
            let is_active = active_hold_idx == Some(idx);
            let progress = if is_active { hold_progress.unwrap_or(0.0) } else { 0.0 };
            if let Some(a) = render_row(ui, idx, text, *ts, is_active, hold_fired, progress) {
                actions.push(a);
            }
            ui.add_space(2.0);
        }

        for a in actions {
            match a {
                RowAction::Append(i) => self.append_to_input(i),
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_window_level(ctx);

        let enter_send = ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
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
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
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

        let need_animation = self.typing_active || self.status.is_some() || self.hold.is_some();
        if need_animation {
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
                    .inner_margin(egui::Margin::symmetric(14.0, 0.0))
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
                                    self.tts_devices = self.tts.list_devices();
                                    self.tts_voices = self.tts.list_voice_names();
                                }
                            }

                            ui.add_space(6.0);

                            // TTS toggle (disabled if SAPI unavailable)
                            let tts_avail = self.tts.available();
                            let tts_tip = if tts_avail {
                                "语音朗读 (Windows SAPI)"
                            } else {
                                "系统 SAPI 不可用"
                            };
                            ui.add_enabled_ui(tts_avail, |ui| {
                                let r = icon_button(ui, "🔊", self.config.tts_enabled, tts_tip);
                                if r.clicked() {
                                    self.config.tts_enabled = !self.config.tts_enabled;
                                    if !self.config.tts_enabled {
                                        self.tts.stop();
                                    } else {
                                        self.set_status("提示：需配合虚拟音频线缆");
                                    }
                                    self.config.save();
                                }
                            });

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
                        left: 12.0,
                        right: 12.0,
                        top: 10.0,
                        bottom: 12.0,
                    })
                    .stroke(egui::Stroke::new(1.0, theme::BORDER)),
            )
            .show(ctx, |ui| {
                // Framed multiline input
                theme::input_frame().show(ui, |ui| {
                    let edit = egui::TextEdit::multiline(&mut self.text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2)
                        .hint_text(
                            "输入消息  ·  [Enter] 发送  ·  [Shift+Enter] 换行",
                        )
                        .frame(false);
                    let response = ui.add_sized([ui.available_width(), 52.0], edit);
                    if !self.focus_requested {
                        response.request_focus();
                        self.focus_requested = true;
                    }
                });

                self.update_typing();

                ui.add_space(8.0);

                ui.horizontal(|ui| {
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
                            .size(12.0)
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
                            ui.label(egui::RichText::new(msg).size(12.0).color(c));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Accent-colored send button
                        let send = egui::Button::new(
                            egui::RichText::new("发送  ⏎")
                                .color(egui::Color32::WHITE)
                                .size(13.0)
                                .strong(),
                        )
                        .fill(theme::ACCENT)
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
                        .min_size(egui::vec2(96.0, 30.0));
                        if ui.add(send).clicked() {
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
                        ui.label(label_text("IP 地址"));
                        ui.add_sized(
                            [200.0, 24.0],
                            egui::TextEdit::singleline(&mut self.ip_input)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.end_row();

                        ui.label(label_text("端口"));
                        ui.add_sized(
                            [200.0, 24.0],
                            egui::TextEdit::singleline(&mut self.port_input)
                                .font(egui::TextStyle::Monospace),
                        );
                        ui.end_row();
                    });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let save = egui::Button::new(
                        egui::RichText::new("保存")
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .fill(theme::ACCENT)
                    .rounding(egui::Rounding::same(theme::ROUNDING_MD))
                    .min_size(egui::vec2(72.0, 28.0));
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
                if !self.tts.available() {
                    ui.label(
                        egui::RichText::new("系统 SAPI 不可用")
                            .color(theme::DANGER),
                    );
                } else {
                    egui::Grid::new("voice_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            // Device
                            ui.label(label_text("输出设备"));
                            let current_label: String = self
                                .config
                                .tts_device_name
                                .clone()
                                .unwrap_or_else(|| DEFAULT_DEVICE_LABEL.to_string());
                            let mut selected_name: Option<String> =
                                self.config.tts_device_name.clone();
                            let mut changed = false;
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("tts_device_combo")
                                    .selected_text(&current_label)
                                    .width(200.0)
                                    .show_ui(ui, |ui| {
                                        for device in &self.tts_devices {
                                            let this_name = if device.id == DEVICE_DEFAULT {
                                                None
                                            } else {
                                                Some(device.name.clone())
                                            };
                                            let is_selected = selected_name == this_name;
                                            if ui
                                                .selectable_label(is_selected, &device.name)
                                                .clicked()
                                                && !is_selected
                                            {
                                                selected_name = this_name;
                                                changed = true;
                                            }
                                        }
                                    });
                            });
                            ui.end_row();

                            // Voice
                            ui.label(label_text("语音 / 语言"));
                            let voice_current: String = self
                                .config
                                .tts_voice_name
                                .clone()
                                .unwrap_or_else(|| DEFAULT_VOICE_LABEL.to_string());
                            let mut selected_voice: Option<String> =
                                self.config.tts_voice_name.clone();
                            let mut voice_changed = false;
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("tts_voice_combo")
                                    .selected_text(&voice_current)
                                    .width(200.0)
                                    .show_ui(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                selected_voice.is_none(),
                                                DEFAULT_VOICE_LABEL,
                                            )
                                            .clicked()
                                            && selected_voice.is_some()
                                        {
                                            selected_voice = None;
                                            voice_changed = true;
                                        }
                                        for voice_name in &self.tts_voices {
                                            let this = Some(voice_name.clone());
                                            let is_selected = selected_voice == this;
                                            if ui
                                                .selectable_label(is_selected, voice_name)
                                                .clicked()
                                                && !is_selected
                                            {
                                                selected_voice = this;
                                                voice_changed = true;
                                            }
                                        }
                                    });
                            });
                            ui.end_row();

                            // Apply device/voice changes
                            if changed {
                                let applied = self
                                    .tts
                                    .apply_device_by_name(selected_name.as_deref());
                                self.config.tts_device_name = applied;
                                self.config.save();
                                self.set_status("TTS 输出设备已切换");
                            }
                            if voice_changed {
                                let applied = self
                                    .tts
                                    .apply_voice_by_name(selected_voice.as_deref());
                                self.config.tts_voice_name = applied;
                                self.config.save();
                                self.set_status("TTS 语音已切换");
                            }
                        });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
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
                            self.tts_devices = self.tts.list_devices();
                            self.tts_voices = self.tts.list_voice_names();
                        }
                    });

                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(
                            "💡 想让队友听到：选一个虚拟音频线缆（如 CABLE Input），\
                             并将 VRChat 麦克风设为该线缆的 Output 端。",
                        )
                        .size(11.0)
                        .color(theme::TEXT_WEAK),
                    );
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

fn render_row(
    ui: &mut egui::Ui,
    idx: usize,
    text: &str,
    ts: i64,
    is_active_hold: bool,
    hold_fired: bool,
    progress: f32,
) -> Option<RowAction> {
    let row_id = egui::Id::new(("vrctext_row_hover", idx));
    let prev_hover = ui
        .ctx()
        .memory(|m| m.data.get_temp::<bool>(row_id))
        .unwrap_or(false);
    let highlighted = prev_hover || is_active_hold;

    let mut action: Option<RowAction> = None;

    let row_fill = if highlighted {
        theme::BG_HOVER
    } else {
        egui::Color32::TRANSPARENT
    };

    let frame = egui::Frame::none()
        .fill(row_fill)
        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0));

    let row_resp = frame
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_min_height(22.0);

                // Timestamp column
                ui.add_sized(
                    [78.0, 20.0],
                    egui::Label::new(
                        egui::RichText::new(format_timestamp(ts))
                            .size(11.0)
                            .color(theme::TEXT_WEAK),
                    ),
                );

                ui.add_space(6.0);

                // Text column
                let text_w = (ui.available_width() - 36.0).max(40.0);
                let text_label = egui::Label::new(
                    egui::RichText::new(text)
                        .size(13.0)
                        .color(theme::TEXT_PRIMARY),
                )
                .truncate()
                .sense(egui::Sense::hover());
                let text_resp = ui.add_sized([text_w, 20.0], text_label);
                if text.chars().count() > 30 {
                    text_resp.on_hover_text(text);
                }

                // Resend button slot (right)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if highlighted {
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
                            egui::RichText::new("↺").size(14.0).color(theme::TEXT_PRIMARY),
                        )
                        .fill(btn_fill)
                        .rounding(egui::Rounding::same(theme::ROUNDING_MD))
                        .stroke(egui::Stroke::new(1.0, theme::BORDER_STRONG))
                        .min_size(egui::vec2(28.0, 22.0));
                        let resp = ui.add(btn).on_hover_text(
                            "短按：添加到输入框\n长按 2 秒：直接重发",
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
                        ui.add_space(28.0);
                    }
                });
            });
        })
        .response;

    let now_hover = row_resp.contains_pointer();
    ui.ctx()
        .memory_mut(|m| m.data.insert_temp(row_id, now_hover));
    if now_hover != prev_hover {
        ui.ctx().request_repaint();
    }

    action
}

fn format_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return "—".into();
    }
    use chrono::{Local, TimeZone};
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else { return "—".into(); };
    let now = Local::now();
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

// Bring Datelike into scope for year() used above.
use chrono::Datelike;

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
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".into(), egui::FontData::from_owned(bytes));
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                fam.insert(0, "cjk".into());
            }
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                fam.push("cjk".into());
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}

// Keep config module referenced so `HistoryEntry` type isn't pruned when unused.
#[allow(dead_code)]
fn _type_check() -> HistoryEntry {
    HistoryEntry { text: String::new(), ts: config::now_ts() }
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
