use glam::{UVec2, Vec2, Vec3};

use crate::{
    canvas::Canvas,
    frame::Frame,
    math::{NEON, saturate},
    text,
};

const L_FPS: [u32; 3] = text::codes("FPS");
const L_MS: [u32; 2] = text::codes("MS");
const L_LVL: [u32; 3] = text::codes("LVL");

const PANEL_W: u32 = 200;
const PANEL_H: u32 = 80;

#[inline(always)]
pub fn stats(cv: &mut Canvas, f: &Frame, level: f32) {
    if f.px.x >= PANEL_W || f.px.y >= PANEL_H {
        return;
    }

    let white = Vec3::ONE;
    let dim = Vec3::splat(0.6);
    let accent = NEON.at(f.t * 0.1);

    cv.rrect(
        Vec2::new(8.0, 8.0),
        Vec2::new(180.0, 62.0),
        6.0,
        Vec3::ZERO,
        0.45,
    );

    cv.text(UVec2::new(16, 16), 2, dim, &L_FPS);
    cv.fixed(UVec2::new(60, 16), 2, white, f.fps, 1);

    cv.text(UVec2::new(16, 30), 2, dim, &L_MS);
    cv.fixed(UVec2::new(60, 30), 2, white, f.ms, 2);

    cv.text(UVec2::new(16, 30), 2, dim, &L_LVL);
    cv.rect(Vec2::new(60.0, 46.0), Vec2::new(116.0, 6.0), white, 0.15);
    cv.rect(
        Vec2::new(60.0, 46.0),
        Vec2::new(116.0 * saturate(level), 6.0),
        accent,
        1.0,
    );
}
