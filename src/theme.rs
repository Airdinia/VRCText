use eframe::egui::{self, Color32, Margin, Rounding, Stroke};

// Slate dark palette
pub const BG_BASE: Color32 = Color32::from_rgb(0x16, 0x19, 0x1E);
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x1E, 0x22, 0x2A);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x26, 0x2B, 0x34);
pub const BG_HOVER: Color32 = Color32::from_rgb(0x2E, 0x33, 0x3E);
pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x36, 0x3C, 0x49);

pub const BORDER: Color32 = Color32::from_rgb(0x2E, 0x33, 0x3E);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x46, 0x4D, 0x5B);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xEF);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA5, 0xAB, 0xB8);
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x6D, 0x74, 0x82);

pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0x9E, 0xFF);

pub const SUCCESS: Color32 = Color32::from_rgb(0x5E, 0xD0, 0x8C);
pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xB8, 0x4D);
pub const DANGER: Color32 = Color32::from_rgb(0xE7, 0x5A, 0x5A);

pub const ROUNDING_MD: f32 = 6.0;
pub const ROUNDING_LG: f32 = 10.0;

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = true;
    v.override_text_color = Some(TEXT_PRIMARY);
    v.panel_fill = BG_BASE;
    v.window_fill = BG_SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    v.window_rounding = Rounding::same(ROUNDING_LG);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };
    v.menu_rounding = Rounding::same(ROUNDING_MD);
    v.popup_shadow = v.window_shadow;

    v.extreme_bg_color = BG_ELEVATED; // TextEdit background
    v.faint_bg_color = BG_SURFACE;
    v.code_bg_color = BG_ELEVATED;
    v.hyperlink_color = ACCENT;

    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    let rounding = Rounding::same(ROUNDING_MD);

    v.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    v.widgets.noninteractive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.noninteractive.rounding = rounding;

    v.widgets.inactive.bg_fill = BG_ELEVATED;
    v.widgets.inactive.weak_bg_fill = BG_ELEVATED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.inactive.rounding = rounding;

    v.widgets.hovered.bg_fill = BG_HOVER;
    v.widgets.hovered.weak_bg_fill = BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.hovered.rounding = rounding;

    v.widgets.active.bg_fill = BG_ACTIVE;
    v.widgets.active.weak_bg_fill = BG_ACTIVE;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.rounding = rounding;

    v.widgets.open.bg_fill = BG_ELEVATED;
    v.widgets.open.weak_bg_fill = BG_ELEVATED;
    v.widgets.open.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.open.rounding = rounding;

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.menu_margin = Margin::same(6.0);
    style.spacing.window_margin = Margin::same(16.0);
    style.spacing.indent = 16.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 0.0;
    style.spacing.interact_size.y = 26.0;

    ctx.set_style(style);
}

/// A subtle card frame — use for grouping related controls in the settings dialog.
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0, BORDER))
        .rounding(Rounding::same(ROUNDING_MD))
        .inner_margin(Margin::symmetric(14.0, 12.0))
}

pub fn input_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_ELEVATED)
        .stroke(Stroke::new(1.0, BORDER_STRONG))
        .rounding(Rounding::same(ROUNDING_MD))
        .inner_margin(Margin::symmetric(12.0, 10.0))
}

/// The primary-action button style (filled with accent, white text). Used for
/// "发送", "保存", etc.
pub fn accent_button(text: &str, min_size: egui::Vec2) -> egui::Button<'_> {
    egui::Button::new(
        egui::RichText::new(text)
            .color(Color32::WHITE)
            .strong(),
    )
    .fill(ACCENT)
    .rounding(Rounding::same(ROUNDING_MD))
    .min_size(min_size)
}
