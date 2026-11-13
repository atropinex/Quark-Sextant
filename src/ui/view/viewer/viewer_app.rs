use eframe::egui::{self, vec2, Align, Align2, Color32, FontId, Id, Key, Layout, RichText, ScrollArea, Stroke, ViewportCommand};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::common::common_interface::{Language, ScanReport, Theme, ThemeColor};
use crate::common::models::graph_metrics_model::GraphMetricsModel;
use crate::common::models::recent_projects_model::RecentProjectsModel;
use crate::common::models::report_export_model::ReportExportModel;
use crate::common::models::settings_model::SettingsModel;
use crate::common::utils::path_helper::{open_path, reveal};
use crate::ui::controller::scan_controller::ScanController;
use crate::ui::controller::theme_controller::{panel_frame, ThemeController};
use crate::ui::view::viewer::graph_canvas::GraphCanvas;
use crate::ui::view::widgets::{button, chip, list_row, score_bar, section_title};

#[derive(PartialEq, Clone, Copy)]
enum LeftTab {
    Scripts,
    Problems,
}

/// The main window: toolbar on top, script list on the left, graph in the middle, inspector on the right.
pub struct ViewerApp {
    settings: SettingsModel,
    recents: RecentProjectsModel,
    themes: ThemeController,
    scan: ScanController,
    report: Option<Arc<ScanReport>>,
    canvas: Option<GraphCanvas>,
    search: String,
    tab: LeftTab,
    grouped: bool,
    notice: Option<(String, f64)>, // a short message in the status bar
}

impl ViewerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, project: Option<PathBuf>) -> Self {
        let settings = SettingsModel::new();
        let themes = ThemeController::new(&cc.egui_ctx, &settings.get().theme);
        let grouped = settings.get().grouped;

        let mut app = Self {
            settings,
            recents: RecentProjectsModel::new(),
            themes,
            scan: ScanController::new(),
            report: None,
            canvas: None,
            search: String::new(),
            tab: LeftTab::Scripts,
            grouped,
            notice: None,
        };

        if let Some(path) = project {
            app.open(path, &cc.egui_ctx);
        }

        app
    }

    fn open(&mut self, path: PathBuf, ctx: &egui::Context) {
        if !path.is_dir() {
            return;
        }

        self.report = None;
        self.canvas = None;
        self.search.clear();
        self.scan.start(path, ctx);
    }

    fn pick_folder(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new().set_title("Select a project folder").pick_folder() {
            self.open(path, ctx);
        }
    }

    /// Picks up a finished scan and builds the graph from it.
    fn poll_scan(&mut self, ctx: &egui::Context) {
        let Some(report) = self.scan.poll() else { return };
        let name = report.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let path = report.root.to_string_lossy().to_string();

        self.recents.add(&report.root, report.kinds.clone());
        self.settings.update(|s| s.last_project = Some(path));
        ctx.send_viewport_cmd(ViewportCommand::Title(format!("{name} - Quark-Sextant")));

        self.canvas = Some(GraphCanvas::new(&report, ctx.input(|i| i.time), self.grouped));
        self.report = Some(Arc::new(report));
    }

    /// F5 rescan, R re-layout, G switch grouped / flow. Ignored while you are typing in the search box.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        let (rescan, relayout, group) = ctx.input(|i| (i.key_pressed(Key::F5), i.key_pressed(Key::R), i.key_pressed(Key::G)));
        let Some(report) = self.report.clone() else { return };

        if rescan {
            self.open(report.root.clone(), ctx);
        } else if let Some(canvas) = self.canvas.as_mut() {
            if relayout {
                canvas.re_layout(&report);
            }

            if group {
                self.grouped = !self.grouped;
                let grouped = self.grouped;
                canvas.set_grouped(&report, grouped);
                self.settings.update(|s| s.grouped = grouped);
            }
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());

        if let Some(path) = dropped.into_iter().filter_map(|f| f.path).find(|p| p.is_dir()) {
            self.open(path, ctx);
        }
    }

    fn export_report(&mut self, ctx: &egui::Context, report: &ScanReport) {
        let name = report.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".to_string());
        let file = rfd::FileDialog::new()
            .set_title("Export report")
            .set_file_name(format!("{name}-sextant-report.md"))
            .add_filter("Markdown", &["md"])
            .save_file();
        let Some(path) = file else { return };

        let text = match fs::write(&path, ReportExportModel::markdown(report)) {
            Ok(_) => format!("Report saved to {}", path.display()),
            Err(error) => format!("Could not save the report: {error}"),
        };

        self.notice = Some((text, ctx.input(|i| i.time)));
    }

    // === Toolbar ================================================================= //

    fn toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: &Theme, report: &Option<Arc<ScanReport>>) {
        let mut picked_theme = None;
        let mut toggle_grouped = false;
        let mut rescan = false;
        let mut export = false;

        ui.horizontal_centered(|ui| {
            if button(ui, theme, "Open Folder", true).clicked() {
                self.pick_folder(ctx);
            }

            if let Some(report) = report {
                ui.add_space(6.0);
                ui.label(RichText::new(report.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).strong().size(14.0));

                for kind in &report.kinds {
                    chip(ui, kind.label(), kind.color());
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button("Theme", |ui| {
                    for item in self.themes.all() {
                        if ui.selectable_label(item.id == theme.id, &item.name).clicked() {
                            picked_theme = Some(item.id.clone());
                            ui.close_menu();
                        }
                    }
                });

                if let (Some(report), Some(canvas)) = (report, self.canvas.as_mut()) {
                    if button(ui, theme, "Export", false).clicked() {
                        export = true;
                    }

                    if button(ui, theme, "Rescan  (F5)", false).clicked() {
                        rescan = true;
                    }

                    if button(ui, theme, if self.grouped { "Grouped  (G)" } else { "Flow  (G)" }, false).clicked() {
                        toggle_grouped = true;
                    }

                    if button(ui, theme, "Re-layout  (R)", false).clicked() {
                        canvas.re_layout(report);
                    }

                    if button(ui, theme, "Fit  (F)", false).clicked() {
                        canvas.fit();
                    }
                }

                ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search scripts").desired_width(190.0));
            });
        });

        if rescan {
            if let Some(report) = report {
                self.open(report.root.clone(), ctx);
            }
        }

        if export {
            if let Some(report) = report {
                self.export_report(ctx, report);
            }
        }

        if toggle_grouped {
            self.grouped = !self.grouped;
            let grouped = self.grouped;

            if let (Some(report), Some(canvas)) = (report, self.canvas.as_mut()) {
                canvas.set_grouped(report, grouped);
            }

            self.settings.update(|s| s.grouped = grouped);
        }

        if let Some(id) = picked_theme {
            self.themes.select(ctx, &id);
            self.settings.update(|s| s.theme = id);
        }
    }

    // === Left panel ============================================================== //

    fn left_panel(&mut self, ui: &mut egui::Ui, theme: &Theme, report: &ScanReport) {
        let large = GraphMetricsModel::large_scripts(&report.graph, 30);
        let problems = report.metrics.cycles.len() + report.metrics.hubs.len() + report.metrics.orphans.len() + large.len();

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, LeftTab::Scripts, "Scripts");
            ui.selectable_value(&mut self.tab, LeftTab::Problems, format!("Problems ({problems})"));
        });
        ui.separator();

        let Some(canvas) = self.canvas.as_mut() else { return };
        let row_height = 40.0 + ui.spacing().item_spacing.y;
        let selected = canvas.selected;
        let mut goto = None;

        if self.tab == LeftTab::Scripts {
            let needle = self.search.trim().to_lowercase();
            let rows: Vec<usize> = (0..report.graph.nodes.len()).filter(|&i| needle.is_empty() || report.graph.nodes[i].rel_path.to_lowercase().contains(&needle)).collect();

            ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_height, rows.len(), |ui, range| {
                for &i in &rows[range] {
                    let node = &report.graph.nodes[i];
                    let folder = if node.folder.is_empty() { "/" } else { node.folder.as_str() };

                    if list_row(ui, theme, &node.name, Some(folder), node.language.color(), selected == Some(i)).clicked() {
                        goto = Some(i);
                    }
                }
            });
        } else {
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if problems == 0 {
                    ui.add_space(12.0);
                    ui.label(RichText::new("No cycles, no hubs. Nice and clean.").color(theme.color(ThemeColor::Dim)));
                }

                for cycle in &report.metrics.cycles {
                    let names: Vec<&str> = cycle.iter().take(3).map(|&i| report.graph.nodes[i].name.as_str()).collect();
                    let title = format!("Circular dependency: {} scripts", cycle.len());
                    let more = if cycle.len() > 3 { ", ..." } else { "" };

                    if list_row(ui, theme, &title, Some(&format!("{}{}", names.join(", "), more)), theme.color(ThemeColor::Danger), cycle.contains(&selected.unwrap_or(usize::MAX))).clicked() {
                        goto = Some(cycle[0]);
                    }
                }

                for &hub in &report.metrics.hubs {
                    let node = &report.graph.nodes[hub];
                    let detail = format!("used by {}, uses {}", report.metrics.fan_in[hub], report.metrics.fan_out[hub]);

                    if list_row(ui, theme, &format!("Hub: {}", node.name), Some(&detail), Color32::from_rgb(0xE0, 0xA8, 0x3C), selected == Some(hub)).clicked() {
                        goto = Some(hub);
                    }
                }

                for &orphan in report.metrics.orphans.iter().take(60) {
                    let node = &report.graph.nodes[orphan];

                    if list_row(ui, theme, &format!("Isolated: {}", node.name), Some("no links, maybe dead code"), theme.color(ThemeColor::Dim), selected == Some(orphan)).clicked() {
                        goto = Some(orphan);
                    }
                }

                for &big in &large {
                    let node = &report.graph.nodes[big];

                    if list_row(ui, theme, &format!("Large: {}", node.name), Some(&format!("{} lines", node.lines)), Color32::from_rgb(0xB6, 0x9C, 0xFF), selected == Some(big)).clicked() {
                        goto = Some(big);
                    }
                }
            });
        }

        if let Some(i) = goto {
            canvas.select(Some(i), true);
        }
    }

    // === Inspector =============================================================== //

    fn inspector(&mut self, ui: &mut egui::Ui, theme: &Theme, report: &ScanReport) {
        let Some(canvas) = self.canvas.as_mut() else { return };
        let dim = theme.color(ThemeColor::Dim);
        let mut goto = None;

        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let Some(s) = canvas.selected else {
                // Nothing selected: show the health of the whole project.
                section_title(ui, theme, "Project health");
                score_bar(ui, theme, Id::new("score"), report.metrics.score, report.metrics.verdict());
                ui.add_space(6.0);
                ui.label(format!("{} scripts, {} links", report.graph.nodes.len(), report.graph.edges.len()));
                ui.label(RichText::new(format!("{} circular groups, {} hubs", report.metrics.cycles.len(), report.metrics.hubs.len())).color(dim));

                section_title(ui, theme, "Languages");
                let mut counts: HashMap<Language, usize> = HashMap::new();

                for node in &report.graph.nodes {
                    *counts.entry(node.language).or_default() += 1;
                }

                let mut counts: Vec<_> = counts.into_iter().collect();
                counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));

                for (language, count) in counts {
                    list_row(ui, theme, &format!("{}   {}", language.label(), count), None, language.color(), false);
                }

                ui.add_space(10.0);
                ui.label(RichText::new("Click a script to follow its flow.\nDouble-click to zoom to it.\nScroll to zoom, drag the background to pan.").size(11.0).color(dim));
                return;
            };

            let node = &report.graph.nodes[s];
            ui.label(RichText::new(&node.name).strong().size(15.0));
            ui.label(RichText::new(&node.rel_path).size(11.0).color(dim));
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                chip(ui, node.language.label(), node.language.color());

                if let Some(c) = report.metrics.cycle_of[s] {
                    chip(ui, &format!("cycle of {}", report.metrics.cycles[c].len()), theme.color(ThemeColor::Danger));
                }

                if report.metrics.hubs.contains(&s) {
                    chip(ui, "hub", Color32::from_rgb(0xE0, 0xA8, 0x3C));
                }
            });

            ui.label(RichText::new(format!("{} lines", node.lines)).color(dim));

            let full = report.root.join(&node.rel_path);

            ui.horizontal_wrapped(|ui| {
                if button(ui, theme, "Open", false).clicked() {
                    open_path(&full);
                }

                if button(ui, theme, "Show folder", false).clicked() {
                    if let Some(folder) = full.parent() {
                        reveal(folder);
                    }
                }

                if button(ui, theme, "Copy path", false).clicked() {
                    let text = full.to_string_lossy().to_string();
                    ui.ctx().output_mut(|o| o.copied_text = text);
                }
            });

            let uses: Vec<usize> = report.graph.edges.iter().filter(|e| e.from == s).map(|e| e.to).collect();
            let used_by: Vec<usize> = report.graph.edges.iter().filter(|e| e.to == s).map(|e| e.from).collect();

            for (title, list) in [(format!("Uses ({})", uses.len()), &uses), (format!("Used by ({})", used_by.len()), &used_by)] {
                section_title(ui, theme, &title);

                for &other in list.iter().take(60) {
                    let target = &report.graph.nodes[other];

                    if list_row(ui, theme, &target.name, None, target.language.color(), false).clicked() {
                        goto = Some(other);
                    }
                }
            }

        });

        if let Some(i) = goto {
            canvas.select(Some(i), true);
        }
    }

    // === Center ================================================================== //

    fn empty_state(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, theme: &Theme) {
        let rect = ui.max_rect();
        let time = ctx.input(|i| i.time) as f32;
        let painter = ui.painter();

        painter.rect_filled(rect, 0.0, theme.color(ThemeColor::Bg));

        let middle = rect.center();
        let (title, hint) = if self.scan.is_busy() {
            let dots = ".".repeat(1 + (time * 2.5) as usize % 3);
            (format!("Scanning{dots}"), format!("{} scripts found", self.scan.found))
        } else {
            ("Select a project folder".to_string(), "Node, API, WebSocket, Unity, Cocos, Unreal and more. Or drop a folder here.".to_string())
        };

        painter.text(middle - vec2(0.0, 36.0), Align2::CENTER_CENTER, title, FontId::proportional(22.0), theme.color(ThemeColor::Text));
        painter.text(middle - vec2(0.0, 8.0), Align2::CENTER_CENTER, hint, FontId::proportional(12.5), theme.color(ThemeColor::Dim));

        if self.scan.is_busy() {
            ctx.request_repaint();

            let bar = egui::Rect::from_center_size(middle + vec2(0.0, 26.0), vec2(220.0, 4.0));
            let sweep = (time * 1.4).fract();
            let head = bar.min.x + bar.width() * sweep;

            painter.rect_filled(bar, 2.0, theme.color(ThemeColor::Field));
            painter.rect_filled(egui::Rect::from_min_max(egui::pos2((head - 60.0).max(bar.min.x), bar.min.y), egui::pos2(head.min(bar.max.x), bar.max.y)), 2.0, theme.color(ThemeColor::Accent));
        } else {
            let area = egui::Rect::from_center_size(middle + vec2(0.0, 30.0), vec2(160.0, 34.0));
            ui.allocate_ui_at_rect(area, |ui| {
                ui.centered_and_justified(|ui| {
                    if button(ui, theme, "Open Folder", true).clicked() {
                        self.pick_folder(ctx);
                    }
                });
            });
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui, theme: &Theme, report: &Option<Arc<ScanReport>>) {
        let dim = theme.color(ThemeColor::Dim);

        ui.horizontal_centered(|ui| {
            if self.scan.is_busy() {
                ui.spinner();
                ui.label(RichText::new(format!("Scanning... {} scripts", self.scan.found)).color(dim).size(11.5));
            } else if let Some(report) = report {
                let m = &report.metrics;
                ui.label(RichText::new(format!("{} scripts   {} links   {} cycles   {}", report.graph.nodes.len(), report.graph.edges.len(), m.cycles.len(), m.verdict())).color(dim).size(11.5));

                if report.graph.truncated {
                    ui.label(RichText::new(format!("Showing the first {} scripts ({} skipped)", report.graph.nodes.len(), report.graph.skipped)).color(theme.color(ThemeColor::Danger)).size(11.5));
                }
            }

            if let Some((text, since)) = &self.notice {
                let age = (ui.ctx().input(|i| i.time) - since) as f32;

                if age < 5.0 {
                    ui.label(RichText::new(text).color(theme.color(ThemeColor::Accent)).size(11.5));
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
                }
            }

            if !self.themes.errors.is_empty() {
                ui.label(RichText::new(format!("Theme problem: {}", self.themes.errors[0])).color(theme.color(ThemeColor::Danger)).size(11.5));
            }

            if let Some(canvas) = &self.canvas {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("Zoom {}%", canvas.zoom_percent())).color(dim).size(11.5));
                });
            }
        });
    }
}

impl eframe::App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_drops(ctx);
        self.poll_scan(ctx);
        self.handle_shortcuts(ctx);

        let theme = self.themes.current();
        let report = self.report.clone();

        egui::TopBottomPanel::top("toolbar").exact_height(42.0).frame(panel_frame(&theme)).show(ctx, |ui| {
            self.toolbar(ui, ctx, &theme, &report);
        });

        egui::TopBottomPanel::bottom("status").exact_height(24.0).frame(panel_frame(&theme)).show(ctx, |ui| {
            self.status_bar(ui, &theme, &report);
        });

        if let Some(report) = &report {
            egui::SidePanel::left("scripts").default_width(260.0).width_range(200.0..=420.0).frame(panel_frame(&theme)).show(ctx, |ui| {
                self.left_panel(ui, &theme, report);
            });

            egui::SidePanel::right("inspector").default_width(270.0).width_range(220.0..=420.0).frame(panel_frame(&theme)).show(ctx, |ui| {
                self.inspector(ui, &theme, report);
            });
        }

        egui::CentralPanel::default().frame(egui::Frame::none().stroke(Stroke::new(1.0, theme.color(ThemeColor::Border)))).show(ctx, |ui| {
            match (&report, self.canvas.as_mut()) {
                (Some(report), Some(canvas)) => canvas.show(ui, &theme, report, &self.search),
                _ => self.empty_state(ui, ctx, &theme),
            }
        });
    }
}
