use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CursorIcon, FontId, Layout, Rect, RichText, Rounding, ScrollArea, Sense, Stroke};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common::common_interface::{Theme, ThemeColor};
use crate::common::models::project_detector_model::ProjectDetectorModel;
use crate::common::models::recent_projects_model::RecentProjectsModel;
use crate::common::models::settings_model::SettingsModel;
use crate::common::utils::color_helper::mix;
use crate::common::utils::ease_helper::ease_out_cubic;
use crate::common::utils::format_helper::time_ago;
use crate::common::utils::path_helper::{reveal, sibling_executable};
use crate::ui::controller::theme_controller::{panel_frame, ThemeController};
use crate::ui::view::widgets::{button, list_row, paint_chip};

#[derive(PartialEq, Clone, Copy)]
enum Page {
    Projects,
    Themes,
}

/// The hub: remembers your projects, picks the theme, and starts the viewer.
pub struct LauncherApp {
    settings: SettingsModel,
    recents: RecentProjectsModel,
    themes: ThemeController,
    page: Page,
    started: f64,
    message: Option<(String, f64)>,
}

impl LauncherApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let settings = SettingsModel::new();
        let themes = ThemeController::new(&cc.egui_ctx, &settings.get().theme);

        Self { settings, recents: RecentProjectsModel::new(), themes, page: Page::Projects, started: 0.0, message: None }
    }

    fn say(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        self.message = Some((text.into(), ctx.input(|i| i.time)));
    }

    fn add_folder(&mut self, ctx: &egui::Context, path: &Path) {
        if path.is_dir() {
            self.recents.add(path, ProjectDetectorModel::detect(path));
            self.say(ctx, "Project added");
        }
    }

    fn launch(&mut self, ctx: &egui::Context, path: &str) {
        let result = Command::new(sibling_executable("quark-sextant")).arg("--project").arg(path).spawn();

        match result {
            Ok(_) => {
                self.recents.add(&PathBuf::from(path), ProjectDetectorModel::detect(Path::new(path)));
                self.say(ctx, "Opening project...");
            }
            Err(error) => self.say(ctx, format!("Could not start the viewer: {error}")),
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        ui.add_space(6.0);
        ui.label(RichText::new("QUARK-SEXTANT").strong().size(15.0).color(theme.color(ThemeColor::Accent)));
        ui.label(RichText::new("see how your scripts really flow").size(11.0).color(theme.color(ThemeColor::Dim)));
        ui.add_space(14.0);

        for (page, label) in [(Page::Projects, "Projects"), (Page::Themes, "Themes")] {
            if list_row(ui, theme, label, None, theme.color(ThemeColor::Accent), self.page == page).clicked() {
                self.page = page;
            }
        }
    }

    fn projects_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: &Theme) {
        let mut add = false;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Projects").size(22.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                add = button(ui, theme, "Add folder", true).clicked();
            });
        });
        ui.add_space(10.0);

        if add {
            if let Some(path) = rfd::FileDialog::new().set_title("Select a project folder").pick_folder() {
                self.add_folder(ctx, &path);
            }
        }

        if self.recents.list().is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No projects yet").size(16.0));
                ui.label(RichText::new("Press \"Add folder\" or drop a project folder on this window.").color(theme.color(ThemeColor::Dim)));
            });
            return;
        }

        let now = ctx.input(|i| i.time);
        let mut open = None;
        let mut remove = None;
        let items = self.recents.list().to_vec();

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, item) in items.iter().enumerate() {
                // Rows slide up and fade in, one after another.
                let t = ease_out_cubic(((now - self.started) as f32 - i as f32 * 0.05) * 3.0);

                let (slot, response) = ui.allocate_exact_size(vec2(ui.available_width(), 68.0), Sense::click());
                ui.add_space(2.0);

                let rect = slot.translate(vec2(0.0, (1.0 - t) * 14.0));
                let hover = ctx.animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
                let painter = ui.painter();
                let fade = |c: Color32| c.gamma_multiply(t);

                painter.rect_filled(rect, Rounding::same(4.0), fade(mix(theme.color(ThemeColor::Field), theme.color(ThemeColor::Select), hover * 0.55)));
                painter.rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0, fade(mix(theme.color(ThemeColor::Border), theme.color(ThemeColor::Accent), hover))));

                let kind_color = item.kinds.first().map(|k| k.color()).unwrap_or(theme.color(ThemeColor::Dim));
                painter.rect_filled(Rect::from_min_size(rect.min, vec2(4.0, rect.height())), Rounding { nw: 4.0, sw: 4.0, ne: 0.0, se: 0.0 }, fade(kind_color));
                painter.text(rect.min + vec2(18.0, 17.0), Align2::LEFT_CENTER, &item.name, FontId::proportional(15.0), fade(theme.color(ThemeColor::Text)));
                painter.text(rect.min + vec2(18.0, 36.0), Align2::LEFT_CENTER, &item.path, FontId::proportional(11.0), fade(theme.color(ThemeColor::Dim)));

                let mut x = rect.min.x + 18.0;

                for kind in &item.kinds {
                    x += paint_chip(painter, pos2(x, rect.min.y + 46.0), kind.label(), fade(kind.color())) + 6.0;
                }

                painter.text(pos2(rect.max.x - 52.0, rect.min.y + 17.0), Align2::RIGHT_CENTER, time_ago(item.last_opened), FontId::proportional(11.0), fade(theme.color(ThemeColor::Dim)));

                let close = Rect::from_center_size(pos2(rect.max.x - 24.0, rect.min.y + 17.0), vec2(22.0, 22.0));
                let close_reply = ui.interact(close, response.id.with("close"), Sense::click());
                let close_hover = ctx.animate_bool_with_time(close_reply.id.with("hover"), close_reply.hovered(), 0.1);

                ui.painter().circle_filled(close.center(), 11.0, theme.color(ThemeColor::Danger).gamma_multiply(0.7 * close_hover * t));
                ui.painter().text(close.center(), Align2::CENTER_CENTER, "x", FontId::proportional(12.0), fade(theme.color(ThemeColor::Text)));

                if close_reply.clicked() {
                    remove = Some(item.path.clone());
                } else if response.clicked() {
                    open = Some(item.path.clone());
                }

                if response.hovered() {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }

                if t < 1.0 {
                    ctx.request_repaint();
                }
            }
        });

        if let Some(path) = remove {
            self.recents.remove(&path);
        } else if let Some(path) = open {
            self.launch(ctx, &path);
        }
    }

    fn themes_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: &Theme) {
        let mut picked = None;
        let mut reload = false;

        ui.horizontal(|ui| {
            ui.label(RichText::new("Themes").size(22.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, theme, "Open themes folder", false).clicked() {
                    reveal(self.themes.folder());
                }

                reload = button(ui, theme, "Reload", false).clicked();
            });
        });

        ui.label(RichText::new("Themes are YAML files. Edit one, press Reload, and the whole app changes.").color(theme.color(ThemeColor::Dim)));
        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            for item in self.themes.all() {
                let (rect, response) = ui.allocate_exact_size(vec2(210.0, 118.0), Sense::click());
                let hover = ctx.animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);
                let chosen = ctx.animate_bool_with_time(response.id.with("chosen"), item.id == theme.id, 0.2);
                let card = rect.expand(hover * 3.0); // grows a little under the cursor
                let color = |c: ThemeColor| item.color(c);
                let painter = ui.painter();

                painter.rect_filled(card, Rounding::same(6.0), color(ThemeColor::Bg));
                painter.rect_filled(Rect::from_min_size(card.min, vec2(card.width(), 26.0)), Rounding { nw: 6.0, ne: 6.0, sw: 0.0, se: 0.0 }, color(ThemeColor::Panel));
                painter.rect_filled(Rect::from_min_size(card.min + vec2(12.0, 40.0), vec2(80.0, 22.0)), Rounding::same(3.0), color(ThemeColor::Node));
                painter.rect_filled(Rect::from_min_size(card.min + vec2(112.0, 58.0), vec2(80.0, 22.0)), Rounding::same(3.0), color(ThemeColor::Node));
                painter.rect_filled(Rect::from_min_size(card.min + vec2(12.0, 40.0), vec2(80.0, 5.0)), Rounding::same(2.0), color(ThemeColor::Accent));
                painter.rect_filled(Rect::from_min_size(card.min + vec2(112.0, 58.0), vec2(80.0, 5.0)), Rounding::same(2.0), color(ThemeColor::Danger));
                painter.rect_filled(Rect::from_min_size(card.min + vec2(12.0, 96.0), vec2(60.0, 10.0)), Rounding::same(2.0), color(ThemeColor::Select));
                painter.text(card.min + vec2(10.0, 13.0), Align2::LEFT_CENTER, &item.name, FontId::proportional(12.0), color(ThemeColor::Text));
                painter.rect_stroke(card, Rounding::same(6.0), Stroke::new(1.0 + chosen * 1.5, mix(theme.color(ThemeColor::Border), theme.color(ThemeColor::Accent), chosen.max(hover * 0.6))));

                if response.clicked() {
                    picked = Some(item.id.clone());
                }
            }
        });

        if let Some(error) = self.themes.errors.first() {
            ui.add_space(8.0);
            ui.label(RichText::new(format!("Theme problem: {error}")).color(theme.color(ThemeColor::Danger)));
        }

        if reload {
            self.themes.reload(ctx);
            self.say(ctx, "Themes reloaded");
        }

        if let Some(id) = picked {
            self.themes.select(ctx, &id);
            self.settings.update(|s| s.theme = id);
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.started == 0.0 {
            self.started = ctx.input(|i| i.time);
        }

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());

        for file in dropped {
            if let Some(path) = file.path {
                self.add_folder(ctx, &path);
            }
        }

        let theme = self.themes.current();

        egui::SidePanel::left("sidebar").exact_width(190.0).resizable(false).frame(panel_frame(&theme)).show(ctx, |ui| {
            self.sidebar(ui, &theme);
        });

        egui::CentralPanel::default().frame(egui::Frame::none().fill(theme.color(ThemeColor::Bg)).inner_margin(egui::Margin::same(20.0))).show(ctx, |ui| {
            match self.page {
                Page::Projects => self.projects_page(ui, ctx, &theme),
                Page::Themes => self.themes_page(ui, ctx, &theme),
            }

            // A short message in the corner that fades away.
            if let Some((text, since)) = &self.message {
                let age = (ctx.input(|i| i.time) - since) as f32;
                let alpha = (1.0 - (age - 2.5) / 0.5).clamp(0.0, 1.0);

                if alpha > 0.0 {
                    let rect = ui.max_rect();
                    ui.painter().text(rect.right_bottom() - vec2(0.0, 4.0), Align2::RIGHT_BOTTOM, text, FontId::proportional(12.0), theme.color(ThemeColor::Accent).gamma_multiply(alpha));
                    ctx.request_repaint();
                }
            }
        });
    }
}
