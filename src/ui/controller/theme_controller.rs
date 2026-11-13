use eframe::egui::{self, vec2, Context, FontId, Rounding, Stroke, TextStyle, Visuals};
use std::path::Path;
use std::sync::Arc;

use crate::common::common_interface::{Theme, ThemeColor};
use crate::common::models::theme_model::ThemeModel;
use crate::common::utils::color_helper::mix;

/// Loads the YAML themes and paints the whole app with the chosen one.
pub struct ThemeController {
    model: ThemeModel,
    themes: Vec<Arc<Theme>>,
    current: Arc<Theme>,
    pub errors: Vec<String>,
}

impl ThemeController {
    pub fn new(ctx: &Context, selected: &str) -> Self {
        let model = ThemeModel::new();
        let list = model.load();
        let themes: Vec<Arc<Theme>> = list.themes.into_iter().map(Arc::new).collect();
        let current = Self::find(&themes, selected);

        apply_theme(ctx, &current);

        Self { model, themes, current, errors: list.errors }
    }

    pub fn current(&self) -> Arc<Theme> {
        self.current.clone()
    }

    pub fn all(&self) -> &[Arc<Theme>] {
        &self.themes
    }

    pub fn folder(&self) -> &Path {
        &self.model.folder
    }

    pub fn select(&mut self, ctx: &Context, id: &str) {
        self.current = Self::find(&self.themes, id);
        apply_theme(ctx, &self.current);
    }

    /// Re-reads the theme files, so you can edit a YAML and see it without restarting.
    pub fn reload(&mut self, ctx: &Context) {
        let list = self.model.load();
        let id = self.current.id.clone();

        self.themes = list.themes.into_iter().map(Arc::new).collect();
        self.errors = list.errors;
        self.select(ctx, &id);
    }

    fn find(themes: &[Arc<Theme>], id: &str) -> Arc<Theme> {
        themes.iter().find(|t| t.id == id).unwrap_or(&themes[0]).clone()
    }
}

pub fn apply_theme(ctx: &Context, theme: &Theme) {
    let c = |color| theme.color(color);
    let (panel, field, border, text) = (c(ThemeColor::Panel), c(ThemeColor::Field), c(ThemeColor::Border), c(ThemeColor::Text));
    let (accent, select) = (c(ThemeColor::Accent), c(ThemeColor::Select));

    let mut visuals = if theme.is_light() { Visuals::light() } else { Visuals::dark() };

    visuals.panel_fill = panel;
    visuals.window_fill = panel;
    visuals.extreme_bg_color = field;
    visuals.faint_bg_color = mix(panel, field, 0.5);
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.hyperlink_color = accent;
    visuals.selection.bg_fill = select;
    visuals.selection.stroke = Stroke::new(1.0, accent);
    visuals.window_rounding = Rounding::same(4.0);
    visuals.menu_rounding = Rounding::same(4.0);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = panel;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, text);

    for (state, fill, line) in [
        (&mut widgets.inactive, field, border),
        (&mut widgets.hovered, mix(field, accent, 0.22), accent),
        (&mut widgets.active, select, accent),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, line);
        state.fg_stroke = Stroke::new(1.0, text);
        state.rounding = Rounding::same(3.0);
    }

    widgets.noninteractive.rounding = Rounding::same(3.0);
    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 4.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style.animation_time = 0.12;
        style.text_styles.insert(TextStyle::Body, FontId::proportional(13.0));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(13.0));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(11.0));
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(18.0));
    });
}

pub fn panel_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::none()
        .fill(theme.color(ThemeColor::Panel))
        .stroke(Stroke::new(1.0, theme.color(ThemeColor::Border)))
        .inner_margin(egui::Margin::symmetric(8.0, 6.0))
}
