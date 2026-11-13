use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Painter, Pos2, Response, Rounding, Sense, Stroke, Ui,
};

use crate::common::common_interface::{Theme, ThemeColor};
use crate::common::utils::color_helper::mix;

/// A button whose color fades in and out instead of snapping.
pub fn button(ui: &mut Ui, theme: &Theme, label: &str, primary: bool) -> Response {
    let text_color = theme.color(ThemeColor::Text);
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(13.0), text_color);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(26.0, 12.0), Sense::click());

    let hover = ui.ctx().animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
    let press = ui.ctx().animate_bool_with_time(response.id.with("press"), response.is_pointer_button_down_on(), 0.06);
    let accent = theme.color(ThemeColor::Accent);
    let base = theme.color(if primary { ThemeColor::Select } else { ThemeColor::Field });
    let fill = mix(mix(base, accent, hover * if primary { 0.30 } else { 0.18 }), Color32::BLACK, press * 0.18);
    let line = mix(theme.color(ThemeColor::Border), accent, hover);

    ui.painter().rect_filled(rect, Rounding::same(3.0), fill);
    ui.painter().rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, line));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, text_color);

    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn paint_chip(painter: &Painter, at: Pos2, text: &str, color: Color32) -> f32 {
    let galley = painter.layout_no_wrap(text.to_string(), FontId::proportional(11.0), color);
    let rect = egui::Rect::from_min_size(at, galley.size() + vec2(14.0, 6.0));

    painter.rect_filled(rect, Rounding::same(8.0), color.gamma_multiply(0.18));
    painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0, color.gamma_multiply(0.5)));
    painter.galley(at + vec2(7.0, 3.0), galley, color);

    rect.width()
}

/// A small colored pill, like Unity's tags.
pub fn chip(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(11.0), color);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(14.0, 6.0), Sense::hover());

    paint_chip(ui.painter(), rect.min, text, color);
}

/// One row of a list, with a soft hover and selection fade. Click it with `.clicked()`.
pub fn list_row(ui: &mut Ui, theme: &Theme, title: &str, subtitle: Option<&str>, dot: Color32, selected: bool) -> Response {
    let height = if subtitle.is_some() { 40.0 } else { 24.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.10);
    let chosen = ui.ctx().animate_bool_with_time(response.id.with("chosen"), selected, 0.15);
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));

    painter.rect_filled(rect, Rounding::same(3.0), theme.color(ThemeColor::Text).gamma_multiply(0.07 * hover));
    painter.rect_filled(rect, Rounding::same(3.0), theme.color(ThemeColor::Select).gamma_multiply(chosen));
    painter.circle_filled(pos2(rect.min.x + 12.0, rect.center().y), 4.0, dot);

    if let Some(subtitle) = subtitle {
        painter.text(pos2(rect.min.x + 26.0, rect.min.y + 12.0), Align2::LEFT_CENTER, title, FontId::proportional(13.0), theme.color(ThemeColor::Text));
        painter.text(pos2(rect.min.x + 26.0, rect.min.y + 28.0), Align2::LEFT_CENTER, subtitle, FontId::proportional(11.0), theme.color(ThemeColor::Dim));
    } else {
        painter.text(pos2(rect.min.x + 26.0, rect.center().y), Align2::LEFT_CENTER, title, FontId::proportional(13.0), theme.color(ThemeColor::Text));
    }

    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn section_title(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.add_space(6.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(10.5).strong().color(theme.color(ThemeColor::Dim)));
    ui.add_space(2.0);
}

/// A bar that slides to its value. Green is clean, red is spaghetti.
pub fn score_bar(ui: &mut Ui, theme: &Theme, id: Id, score: f32, verdict: &str) {
    let value = ui.ctx().animate_value_with_time(id, score / 100.0, 0.8);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    let green = Color32::from_rgb(0x5C, 0xB8, 0x5C);
    let amber = Color32::from_rgb(0xE0, 0xA8, 0x3C);
    let red = theme.color(ThemeColor::Danger);
    let color = if value < 0.5 { mix(green, amber, value * 2.0) } else { mix(amber, red, (value - 0.5) * 2.0) };

    ui.painter().rect_filled(rect, Rounding::same(3.0), theme.color(ThemeColor::Field));
    ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, vec2(rect.width() * value.max(0.02), rect.height())), Rounding::same(3.0), color);
    ui.painter().rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, theme.color(ThemeColor::Border)));
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, format!("{verdict}  {:.0}", score), FontId::proportional(11.0), Color32::WHITE);
}
