use glam::{UVec2, UVec3, Vec2, Vec3, Vec4};
use spirv_std::image::StorageImage2d;

use crate::{canvas::Canvas, math::ShaderParams};

pub struct Frame {
    pub px: UVec2,
    pub res: Vec2,
    pub uv: Vec2,
    pub uv01: Vec2,
    pub t: f32,
    pub dt: f32,
    pub fps: f32,
    pub ms: f32,
    pub mouse01: Vec2,
    pub mouse: Vec2,
    pub mouse_down: bool,
}

impl Frame {
    #[inline(always)]
    pub fn new(id: UVec3, dims: UVec2, p: &ShaderParams) -> Self {
        let px = id.truncate();
        let res = Vec2::new(dims.x as f32, dims.y as f32);
        let uv01 = Vec2::new(
            (px.x as f32 + 0.5) / res.x,
            1.0 - (px.y as f32 + 0.5) / res.y,
        );

        let mouse01 = Vec2::new(p.mouse_x, p.mouse_y);
        let aspect = Vec2::new(res.x / res.y, 1.0);

        Self {
            px,
            res,
            uv01,
            uv: (uv01 - Vec2::splat(0.5)) * 2.0 * Vec2::new(res.x / res.y, 1.0),
            t: p.time,
            dt: p.delta_time,
            fps: p.fps,
            ms: p.frame_ms,
            mouse01,
            mouse: (mouse01 - Vec2::splat(0.5)) * 2.0 * aspect,
            mouse_down: p.mouse_down > 0.5,
        }
    }

    #[inline(always)]
    pub fn canvas(&self, col: Vec3) -> Canvas {
        Canvas::new(self.px, col)
    }

    #[inline(always)]
    pub fn write(&self, out: &StorageImage2d, cv: Canvas) {
        let col = cv.finish();
        unsafe {
            out.write(self.px, Vec4::new(col.x, col.y, col.z, 1.0));
        }
    }
}
