use eframe::egui::Color32;

/// "#rrggbb" or "#rrggbbaa".
pub fn hex(text: &str) -> Option<Color32> {
    let t = text.trim().trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(t.get(i..i + 2)?, 16).ok();

    match t.len() {
        6 => Some(Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

/// Blends `a` into `b`. 0 gives `a`, 1 gives `b`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;

    Color32::from_rgba_unmultiplied(
        channel(a.r(), b.r()),
        channel(a.g(), b.g()),
        channel(a.b(), b.b()),
        channel(a.a(), b.a()),
    )
}
