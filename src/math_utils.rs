use glam::{Vec2, vec2};

/// Rotate `pos` around the origin (0, 0) by `radians`.
pub fn rotate_point(pos: Vec2, radians: f32) -> Vec2 {
    let (cos_theta, sin_theta) = (radians.cos(), radians.sin());
    vec2(
        cos_theta * pos.x - sin_theta * pos.y,
        sin_theta * pos.x + cos_theta * pos.y,
    )
}

/// Where `position` falls in `[min, max]`, as a clamped 0..1 value.
pub fn percent_in_range(position: f32, min: f32, max: f32) -> f32 {
    ((position - min) / (max - min)).clamp(0.0, 1.0)
}
