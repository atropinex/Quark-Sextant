use eframe::egui::{Pos2, Rect, Vec2};

use crate::common::utils::ease_helper::smooth;

const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 2.5;

/// The view onto the graph. Pan and zoom never jump: they glide toward a target.
pub struct CameraController {
    pub pan: Vec2,
    pub zoom: f32,
    target_pan: Vec2,
    target_zoom: f32,
}

impl CameraController {
    pub fn new() -> Self {
        Self { pan: Vec2::ZERO, zoom: 1.0, target_pan: Vec2::ZERO, target_zoom: 1.0 }
    }

    pub fn to_screen(&self, origin: Pos2, world: Pos2) -> Pos2 {
        origin + world.to_vec2() * self.zoom + self.pan
    }

    /// Moves right now (used while dragging the background).
    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
        self.target_pan += delta;
    }

    /// Zooms toward a point so the thing under the cursor stays under the cursor.
    pub fn zoom_at(&mut self, local: Vec2, factor: f32) {
        let zoom = (self.target_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let world = (local - self.target_pan) / self.target_zoom;

        self.target_zoom = zoom;
        self.target_pan = local - world * zoom;
    }

    pub fn fit(&mut self, bounds: Rect, view: Vec2) {
        let zoom = (view.x / (bounds.width() + 160.0)).min(view.y / (bounds.height() + 160.0)).clamp(MIN_ZOOM, 1.2);

        self.target_zoom = zoom;
        self.target_pan = view / 2.0 - bounds.center().to_vec2() * zoom;
    }

    pub fn focus(&mut self, world_center: Pos2, view: Vec2) {
        let zoom = self.target_zoom.clamp(0.7, 1.1);

        self.target_zoom = zoom;
        self.target_pan = view / 2.0 - world_center.to_vec2() * zoom;
    }

    /// Returns true while still moving, so the caller knows to keep drawing frames.
    pub fn step(&mut self, dt: f32) -> bool {
        let k = smooth(dt, 12.0);

        self.pan += (self.target_pan - self.pan) * k;
        self.zoom += (self.target_zoom - self.zoom) * k;

        let moving = (self.target_pan - self.pan).length() > 0.1 || (self.target_zoom - self.zoom).abs() > 0.0005;

        if !moving {
            self.pan = self.target_pan;
            self.zoom = self.target_zoom;
        }

        moving
    }
}
