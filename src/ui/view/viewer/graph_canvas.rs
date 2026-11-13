use eframe::egui::{pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Key, Painter, Pos2, Rect, Rounding, Sense, Shape, Stroke, Ui, Vec2};

use crate::common::common_interface::{ScanReport, Theme, ThemeColor};
use crate::common::models::layout_model::{LayoutModel, NODE_H, NODE_W};
use crate::common::utils::color_helper::mix;
use crate::common::utils::ease_helper::{approach, ease_out_cubic, lerp, smooth};
use crate::ui::controller::camera_controller::CameraController;

const HEADER_H: f32 = 24.0;
const PORT_Y: f32 = 0.62; // how far down the node the in / out dots sit

/// The draggable node graph. Nodes glide to their place, the camera glides to its target,
/// and selecting a script fades everything unrelated into the background.
pub struct GraphCanvas {
    pub selected: Option<usize>,
    pos: Vec<Pos2>,    // where each node is drawn now
    target: Vec<Pos2>, // where it is heading
    neighbors: Vec<Vec<usize>>,
    related: Vec<bool>,
    lower_paths: Vec<String>,
    camera: CameraController,
    focus: f32, // 0 = nothing selected, 1 = everything unrelated is dimmed
    born: f64,
    view: Vec2,
    needs_fit: bool,
    grouped: bool,
    groups: Vec<(String, Vec<usize>)>, // folder name and its scripts
}

impl GraphCanvas {
    pub fn new(report: &ScanReport, now: f64, grouped: bool) -> Self {
        let count = report.graph.nodes.len();
        let target = Self::layout(report, grouped);
        let mut neighbors = vec![Vec::new(); count];

        for edge in &report.graph.edges {
            neighbors[edge.from].push(edge.to);
            neighbors[edge.to].push(edge.from);
        }

        // Everything starts in the middle and blooms outward.
        let center = bounds(&target).center();

        Self {
            selected: None,
            pos: vec![center; count],
            target,
            neighbors,
            related: vec![false; count],
            lower_paths: report.graph.nodes.iter().map(|n| n.rel_path.to_lowercase()).collect(),
            camera: CameraController::new(),
            focus: 0.0,
            born: now,
            view: vec2(1000.0, 700.0),
            needs_fit: true,
            grouped,
            groups: LayoutModel::folder_groups(&report.graph),
        }
    }

    fn layout(report: &ScanReport, grouped: bool) -> Vec<Pos2> {
        let points = if grouped { LayoutModel::arrange_grouped(&report.graph) } else { LayoutModel::arrange(&report.graph, &report.metrics) };
        points.into_iter().map(|p| pos2(p[0], p[1])).collect()
    }

    pub fn re_layout(&mut self, report: &ScanReport) {
        self.target = Self::layout(report, self.grouped);
        self.needs_fit = true;
    }

    /// Switches between boxes-by-folder and the flow layout. Nodes glide to their new places.
    pub fn set_grouped(&mut self, report: &ScanReport, grouped: bool) {
        self.grouped = grouped;
        self.re_layout(report);
    }

    pub fn fit(&mut self) {
        self.camera.fit(bounds(&self.target), self.view);
    }

    pub fn zoom_percent(&self) -> i32 {
        (self.camera.zoom * 100.0).round() as i32
    }

    pub fn select(&mut self, index: Option<usize>, focus_camera: bool) {
        self.selected = index;
        self.related.iter_mut().for_each(|r| *r = false);

        if let Some(i) = index {
            self.related[i] = true;

            for &other in &self.neighbors[i] {
                self.related[other] = true;
            }

            if focus_camera {
                self.camera.focus(self.target[i] + vec2(NODE_W, NODE_H) / 2.0, self.view);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui, theme: &Theme, report: &ScanReport, filter: &str) {
        let ctx = ui.ctx().clone();
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        self.view = rect.size();

        if self.needs_fit {
            self.fit();
            self.needs_fit = false;
        }

        let (dt, time, hover) = ctx.input(|i| (i.stable_dt.min(0.05), i.time, i.pointer.hover_pos()));

        // --- input ---------------------------------------------------------------- //
        if response.dragged() {
            self.camera.pan_by(response.drag_delta());
        }

        if let Some(cursor) = hover.filter(|p| rect.contains(*p)) {
            let (scroll, pinch) = ctx.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));

            if scroll != 0.0 {
                self.camera.zoom_at(cursor - rect.min, (scroll * 0.0015).exp());
            }

            if pinch != 1.0 {
                self.camera.zoom_at(cursor - rect.min, pinch);
            }
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.fit();
            }

            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.select(None, false);
            }
        }

        // --- motion --------------------------------------------------------------- //
        let camera_moving = self.camera.step(dt);
        let k = smooth(dt, 9.0);
        let mut nodes_moving = false;

        for i in 0..self.pos.len() {
            let delta = self.target[i] - self.pos[i];

            if delta.length_sq() > 0.25 {
                self.pos[i] += delta * k;
                nodes_moving = true;
            } else {
                self.pos[i] = self.target[i];
            }
        }

        let want_focus = if self.selected.is_some() { 1.0 } else { 0.0 };
        self.focus = approach(self.focus, want_focus, dt, 10.0);
        let age = (time - self.born) as f32;

        // --- where everything is on screen ---------------------------------------- //
        let zoom = self.camera.zoom;
        let origin = rect.min;
        let size = vec2(NODE_W, NODE_H) * zoom;
        let top_left: Vec<Pos2> = self.pos.iter().map(|p| self.camera.to_screen(origin, *p)).collect();
        let on_screen = rect.expand(80.0);

        // --- nodes: click, drag, hover (done first so they sit above the background) --- //
        let mut visible: Vec<(usize, Rect, f32, bool)> = Vec::new();
        let mut clicked = None;
        let mut double_clicked = None;

        for i in 0..top_left.len() {
            let appear = ease_out_cubic((age - (i as f32 * 0.003).min(0.6)) * 3.0);
            let node_rect = Rect::from_center_size(Rect::from_min_size(top_left[i], size).center(), size * (0.85 + 0.15 * appear));

            if appear <= 0.0 || !on_screen.intersects(node_rect) {
                continue;
            }

            let reply = ui.interact(node_rect, response.id.with(("node", i)), Sense::click_and_drag());

            if reply.drag_started() || reply.clicked() {
                clicked = Some(i);
            }

            if reply.double_clicked() {
                double_clicked = Some(i);
            }

            if reply.dragged() {
                let delta = reply.drag_delta() / zoom;
                self.pos[i] += delta;
                self.target[i] += delta;
                ctx.set_cursor_icon(CursorIcon::Grabbing);
            } else if reply.hovered() {
                ctx.set_cursor_icon(CursorIcon::Grab);
            }

            visible.push((i, node_rect, appear, reply.hovered()));
        }

        // --- paint ----------------------------------------------------------------- //
        painter.rect_filled(rect, 0.0, theme.color(ThemeColor::Bg));
        paint_grid(&painter, rect, &self.camera, theme.color(ThemeColor::Grid));

        let edges_in = ease_out_cubic((age - 0.25) * 2.0);

        if self.grouped {
            for (name, members) in &self.groups {
                let mut area = Rect::NOTHING;

                for &m in members {
                    area.extend_with(top_left[m]);
                    area.extend_with(top_left[m] + size);
                }

                let area = Rect::from_min_max(area.min - vec2(18.0, 38.0) * zoom, area.max + vec2(18.0, 18.0) * zoom);

                if !on_screen.intersects(area) {
                    continue;
                }

                let tint = group_tint(name);
                let touched = members.iter().any(|&m| self.related[m]);
                let alpha = edges_in * if self.selected.is_some() && !touched { lerp(1.0, 0.3, self.focus) } else { 1.0 };
                let round = Rounding::same((8.0 * zoom).clamp(3.0, 12.0));
                let title = format!("{}   {}", if name.is_empty() { "/" } else { name.as_str() }, members.len());

                painter.rect_filled(area, round, mix(theme.color(ThemeColor::Bg), tint, 0.10).gamma_multiply(alpha));
                painter.rect_stroke(area, round, Stroke::new(1.0, tint.gamma_multiply(0.35 * alpha)));
                painter
                    .with_clip_rect(area.intersect(rect))
                    .text(area.min + vec2(12.0, 18.0 * zoom + 2.0), Align2::LEFT_CENTER, title, FontId::proportional((14.0 * zoom).max(10.0)), tint.gamma_multiply(alpha));
            }
        }

        for pass in 0..2 {
            for (index, edge) in report.graph.edges.iter().enumerate() {
                let linked = self.selected.map_or(false, |s| edge.from == s || edge.to == s);

                if (pass == 1) != linked {
                    continue; // linked edges are drawn last, on top
                }

                let from = top_left[edge.from] + vec2(size.x, size.y * PORT_Y);
                let to = top_left[edge.to] + vec2(0.0, size.y * PORT_Y);

                if !on_screen.intersects(Rect::from_two_pos(from, to)) {
                    continue;
                }

                let in_cycle = report.metrics.edge_in_cycle[index];
                let color = if in_cycle {
                    theme.color(ThemeColor::Danger)
                } else if linked {
                    theme.color(ThemeColor::Accent)
                } else {
                    theme.color(ThemeColor::Dim)
                };
                let alpha = if linked { 1.0 } else { lerp(if in_cycle { 0.8 } else { 0.5 }, 0.06, self.focus) } * edges_in;
                let width = (if linked { 2.4 } else { 1.4 }) * zoom.clamp(0.6, 1.4);
                let reach = ((to.x - from.x).abs() * 0.5).max(70.0 * zoom);
                let segments = if zoom < 0.4 { 8 } else { 16 };

                painter.add(Shape::line(
                    bezier(from, from + vec2(reach, 0.0), to - vec2(reach, 0.0), to, segments),
                    Stroke::new(width, color.gamma_multiply(alpha)),
                ));
            }
        }

        let needle = filter.trim().to_lowercase();

        for &(i, node_rect, appear, hovered) in &visible {
            let matches = needle.is_empty() || self.lower_paths[i].contains(&needle);
            let dimmed = self.selected.is_some() && !self.related[i];
            let alpha = appear * if dimmed { lerp(1.0, 0.25, self.focus) } else { 1.0 } * if matches { 1.0 } else { 0.2 };
            let hover_t = ctx.animate_bool_with_time(Id::new(("node-hover", i)), hovered, 0.12);
            let select_t = ctx.animate_bool_with_time(Id::new(("node-select", i)), self.selected == Some(i), 0.18);

            paint_node(&painter, theme, report, i, node_rect, zoom, alpha, hover_t, select_t);
        }

        // --- events ---------------------------------------------------------------- //
        if let Some(i) = double_clicked {
            self.select(Some(i), true);
        } else if let Some(i) = clicked {
            self.select(Some(i), false);
        } else if response.clicked() {
            self.select(None, false);
        }

        let focus_moving = (self.focus - want_focus).abs() > 0.003;

        if camera_moving || nodes_moving || focus_moving || age < 1.6 {
            ctx.request_repaint();
        }
    }
}

/// A calm, stable color per folder name.
fn group_tint(name: &str) -> Color32 {
    const PALETTE: [(u8, u8, u8); 6] = [(0x5A, 0xA0, 0xE6), (0x6C, 0xC2, 0x4A), (0xE0, 0xA8, 0x3C), (0xB6, 0x9C, 0xFF), (0x4C, 0xC9, 0xB0), (0xE0, 0x7A, 0x7A)];
    let hash = name.bytes().fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
    let (r, g, b) = PALETTE[hash % PALETTE.len()];

    Color32::from_rgb(r, g, b)
}

fn bounds(points: &[Pos2]) -> Rect {
    if points.is_empty() {
        return Rect::from_min_size(Pos2::ZERO, vec2(NODE_W, NODE_H));
    }

    let mut rect = Rect::NOTHING;

    for p in points {
        rect.extend_with(*p);
        rect.extend_with(*p + vec2(NODE_W, NODE_H));
    }

    rect
}

fn bezier(a: Pos2, b: Pos2, c: Pos2, d: Pos2, segments: usize) -> Vec<Pos2> {
    (0..=segments)
        .map(|s| {
            let t = s as f32 / segments as f32;
            let u = 1.0 - t;
            let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];

            pos2(
                weights[0] * a.x + weights[1] * b.x + weights[2] * c.x + weights[3] * d.x,
                weights[0] * a.y + weights[1] * b.y + weights[2] * c.y + weights[3] * d.y,
            )
        })
        .collect()
}

/// Small squares and big squares. The small ones fade out as you zoom away.
fn paint_grid(painter: &Painter, rect: Rect, camera: &CameraController, color: Color32) {
    for (step, strength) in [(24.0, 0.7), (120.0, 1.4)] {
        let gap = step * camera.zoom;

        if gap < 7.0 {
            continue;
        }

        let stroke = Stroke::new(1.0, color.gamma_multiply((strength * ((gap - 7.0) / 10.0).clamp(0.0, 1.0)).min(1.0)));
        let mut x = rect.min.x + camera.pan.x.rem_euclid(gap);

        while x < rect.max.x {
            painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], stroke);
            x += gap;
        }

        let mut y = rect.min.y + camera.pan.y.rem_euclid(gap);

        while y < rect.max.y {
            painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], stroke);
            y += gap;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_node(painter: &Painter, theme: &Theme, report: &ScanReport, i: usize, rect: Rect, zoom: f32, alpha: f32, hover: f32, select: f32) {
    let node = &report.graph.nodes[i];
    let in_cycle = report.metrics.cycle_of[i].is_some();
    let corner = (5.0 * zoom).clamp(2.0, 8.0);
    let accent = theme.color(ThemeColor::Accent);
    let danger = theme.color(ThemeColor::Danger);
    let fade = |c: Color32| c.gamma_multiply(alpha);

    painter.rect_filled(rect.translate(vec2(2.0, 4.0) * zoom), Rounding::same(corner), fade(Color32::from_black_alpha(70)));
    painter.rect_filled(rect, Rounding::same(corner), fade(mix(theme.color(ThemeColor::Node), accent, hover * 0.10)));

    if select > 0.01 {
        painter.rect_stroke(rect.expand(3.0 * zoom), Rounding::same(corner + 3.0), Stroke::new(4.0 * zoom.max(0.5), fade(accent.gamma_multiply(0.25 * select))));
    }

    let border = if select > 0.0 {
        mix(theme.color(ThemeColor::Border), accent, select)
    } else if in_cycle {
        mix(theme.color(ThemeColor::Border), danger, 0.85)
    } else {
        mix(theme.color(ThemeColor::Border), accent, hover * 0.6)
    };

    let header = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_H * zoom));
    let top_round = Rounding { nw: corner, ne: corner, sw: 0.0, se: 0.0 };

    painter.rect_filled(header, top_round, fade(mix(theme.color(ThemeColor::Node), node.language.color(), 0.8)));
    painter.rect_stroke(rect, Rounding::same(corner), Stroke::new(1.0 + select * 1.2, fade(border)));

    if zoom > 0.45 {
        let clip = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        let pad = 9.0 * zoom;
        let text = theme.color(ThemeColor::Text);
        let dim = theme.color(ThemeColor::Dim);

        clip.text(pos2(rect.min.x + pad, header.center().y), Align2::LEFT_CENTER, &node.name, FontId::proportional(12.0 * zoom), fade(Color32::WHITE));
        clip.text(pos2(rect.min.x + pad, rect.min.y + (HEADER_H + 14.0) * zoom), Align2::LEFT_CENTER, if node.folder.is_empty() { "/" } else { node.folder.as_str() }, FontId::proportional(10.0 * zoom), fade(dim));
        clip.text(
            pos2(rect.min.x + pad, rect.min.y + (HEADER_H + 29.0) * zoom),
            Align2::LEFT_CENTER,
            format!("in {}  out {}  {} lines", report.metrics.fan_in[i], report.metrics.fan_out[i], node.lines),
            FontId::proportional(10.0 * zoom),
            fade(text.gamma_multiply(0.8)),
        );
    }

    let port_y = rect.min.y + rect.height() * PORT_Y;
    let radius = (4.5 * zoom).clamp(2.0, 6.0);

    for (x, used) in [(rect.min.x, report.metrics.fan_in[i] > 0), (rect.max.x, report.metrics.fan_out[i] > 0)] {
        let fill = if used { accent } else { theme.color(ThemeColor::Dim).gamma_multiply(0.5) };

        painter.circle_filled(pos2(x, port_y), radius, fade(fill));
        painter.circle_stroke(pos2(x, port_y), radius, Stroke::new(1.0, fade(theme.color(ThemeColor::Border))));
    }
}
