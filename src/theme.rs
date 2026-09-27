use egui::{Color32, CornerRadius, Margin, Stroke, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x0b, 0x0b, 0x0e);
pub const SURFACE: Color32 = Color32::from_rgb(0x11, 0x11, 0x14);
pub const PANEL: Color32 = Color32::from_rgb(0x15, 0x15, 0x1a);
pub const BORDER: Color32 = Color32::from_rgb(0x23, 0x23, 0x29);
pub const BORDER2: Color32 = Color32::from_rgb(0x33, 0x33, 0x3c);
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0x8a, 0x3d);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xe6, 0x75, 0x2f);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x16, 0x0a, 0x02);
pub const GREEN: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
pub const RED: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
pub const YELLOW: Color32 = Color32::from_rgb(0xfa, 0xcc, 0x15);
pub const BLUE: Color32 = Color32::from_rgb(0x60, 0xa5, 0xfa);
pub const MUTED: Color32 = Color32::from_rgb(0x6b, 0x6b, 0x76);
pub const SUBTLE: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x21);
pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xec);
pub const TEXT2: Color32 = Color32::from_rgb(0x9c, 0x9c, 0xa6);

pub fn install(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = SUBTLE;
    visuals.faint_bg_color = Color32::from_rgb(0x13, 0x13, 0x18);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.22);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;

    let w = &mut visuals.widgets;
    for s in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        s.corner_radius = CornerRadius::same(7);
        s.fg_stroke.color = TEXT;
    }
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke.color = TEXT2;

    w.inactive.bg_fill = SUBTLE;
    w.inactive.weak_bg_fill = SUBTLE;
    w.inactive.bg_stroke = Stroke::new(1.0, BORDER2);

    w.hovered.bg_fill = Color32::from_rgb(0x24, 0x24, 0x2c);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x24, 0x24, 0x2c);
    w.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.6));

    w.active.bg_fill = Color32::from_rgb(0x2c, 0x2c, 0x36);
    w.active.weak_bg_fill = Color32::from_rgb(0x2c, 0x2c, 0x36);
    w.active.bg_stroke = Stroke::new(1.0, ACCENT);

    w.open.bg_fill = SUBTLE;
    w.open.weak_bg_fill = SUBTLE;
    w.open.bg_stroke = Stroke::new(1.0, BORDER2);

    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.interact_size.y = 26.0;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.floating = false;
    });
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 12))
}

pub fn sunken() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(8))
}
