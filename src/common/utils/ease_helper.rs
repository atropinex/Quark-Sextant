/// How far to move this frame so motion looks the same at 60 or 144 fps.
pub fn smooth(dt: f32, speed: f32) -> f32 {
    1.0 - (-dt * speed).exp()
}

/// Glides `current` toward `target`. Frame-rate independent.
pub fn approach(current: f32, target: f32, dt: f32, speed: f32) -> f32 {
    current + (target - current) * smooth(dt, speed)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}
