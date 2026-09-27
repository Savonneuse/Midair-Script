use egui::{Color32, CornerRadius, RichText, Stroke, Ui, Vec2};

use crate::theme as th;

pub fn section_title(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 13.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(2), th::ACCENT);
        ui.add_space(2.0);
        ui.label(
            RichText::new(title.to_uppercase())
                .size(10.0)
                .strong()
                .color(th::TEXT2),
        );
    });
    ui.add_space(4.0);
}

pub fn micro_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(8.5)
            .strong()
            .color(th::MUTED),
    );
}

pub fn num_field(ui: &mut Ui, value: &mut String, width: f32, valid: bool) -> bool {
    let stroke = if valid {
        Stroke::new(1.0, th::BORDER2)
    } else {
        Stroke::new(1.0, th::RED)
    };
    let color = if valid { th::TEXT } else { th::RED };

    let mut style = ui.style_mut().clone();
    style.visuals.widgets.inactive.bg_stroke = stroke;
    style.visuals.widgets.hovered.bg_stroke = stroke;
    let old = std::mem::replace(ui.style_mut(), style);

    let resp = ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .font(egui::TextStyle::Monospace)
            .text_color(color)
            .margin(egui::Margin::symmetric(7, 5)),
    );

    *ui.style_mut() = old;
    resp.changed()
}

fn paint_button(ui: &mut Ui, fill: Color32, hover: Color32, radius: u8) -> egui::Style {
    let saved = ui.style_mut().clone();
    let v = &mut ui.style_mut().visuals.widgets;
    for (state, bg) in [
        (&mut v.inactive, fill),
        (&mut v.hovered, hover),
        (&mut v.active, hover),
    ] {
        state.weak_bg_fill = bg;
        state.bg_fill = bg;
        state.bg_stroke = Stroke::NONE;
        state.corner_radius = CornerRadius::same(radius);
        state.expansion = 0.0;
    }
    saved
}

pub fn button(ui: &mut Ui, text: &str, primary: bool, size: Vec2) -> egui::Response {
    let (fill, hover, fg) = if primary {
        (th::ACCENT, th::ACCENT_DARK, th::ON_ACCENT)
    } else {
        (th::SUBTLE, th::BORDER2, th::TEXT2)
    };
    let saved = paint_button(ui, fill, hover, 10);
    let resp = ui.add_sized(
        size,
        egui::Button::new(RichText::new(text).strong().color(fg)),
    );
    *ui.style_mut() = saved;
    resp
}

pub fn pill(ui: &mut Ui, dot: Color32, text: &str) {
    const DOT: f32 = 4.5;
    const GAP: f32 = 7.0;

    let galley =
        ui.painter()
            .layout_no_wrap(text.to_owned(), egui::FontId::proportional(11.0), th::TEXT2);
    let size = Vec2::new(
        DOT * 2.0 + GAP + galley.size().x,
        galley.size().y.max(DOT * 2.0),
    );
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    let painter = ui.painter();
    painter.circle_filled(egui::pos2(rect.left() + DOT, rect.center().y), DOT, dot);
    painter.galley(
        egui::pos2(
            rect.left() + DOT * 2.0 + GAP,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        th::TEXT2,
    );
}

pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    current: &mut T,
    options: &[(T, &str)],
    width: f32,
) -> bool {
    let mut changed = false;
    let seg_w = (width - (options.len() as f32 - 1.0) * 4.0) / options.len() as f32;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (value, label) in options {
            let selected = *current == *value;
            let (fill, hover, fg) = if selected {
                (th::ACCENT, th::ACCENT_DARK, th::ON_ACCENT)
            } else {
                (th::SUBTLE, th::BORDER2, th::TEXT2)
            };
            let saved = paint_button(ui, fill, hover, 7);
            let resp = ui.add_sized(
                Vec2::new(seg_w, 28.0),
                egui::Button::new(RichText::new(*label).strong().color(fg)),
            );
            *ui.style_mut() = saved;
            if resp.clicked() && !selected {
                *current = *value;
                changed = true;
            }
        }
    });
    changed
}

pub fn checkbox(ui: &mut Ui, value: &mut bool, label: &str) -> egui::Response {
    ui.style_mut().visuals.widgets.inactive.bg_fill = th::SUBTLE;
    ui.style_mut().visuals.widgets.hovered.bg_fill = th::BORDER2;
    ui.style_mut().visuals.widgets.active.bg_fill = th::ACCENT;
    ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, th::ACCENT);
    ui.style_mut().visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, th::ACCENT);
    ui.style_mut().visuals.widgets.active.fg_stroke = Stroke::new(2.0, th::ON_ACCENT);
    ui.checkbox(value, RichText::new(label).size(11.5).color(th::TEXT2))
}

pub fn kv(ui: &mut Ui, key: &str, value: String, color: Color32) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 15.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.add_sized(
                Vec2::new(62.0, 14.0),
                egui::Label::new(RichText::new(key).size(10.5).color(th::MUTED)),
            );
            ui.label(RichText::new(value).size(11.0).monospace().color(color));
        },
    );
}

pub fn right_aligned<R>(ui: &mut Ui, height: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), height),
        egui::Layout::right_to_left(egui::Align::Center),
        add,
    )
    .inner
}
