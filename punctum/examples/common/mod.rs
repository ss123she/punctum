#![allow(dead_code)]

use glam::Vec2;

pub const MID_GRAY: f32 = 0.5;
pub const TINT_OFFSET: f32 = 0.05;

pub fn tint_channel(channel: f32) -> f32 {
    channel * 0.9 + TINT_OFFSET
}

pub fn half_size(size: Vec2) -> Vec2 {
    size * 0.5
}

pub fn scaled(uv: Vec2, scale: f32) -> Vec2 {
    glam::vec2(uv.x * scale, uv.y * scale)
}
