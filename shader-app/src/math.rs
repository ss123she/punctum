use glam::Vec2;

pub const GRID_TINT: f32 = 0.35;

const UV_SCALE: f32 = 1.0;

pub struct Extents {
    pub w: f32,
    pub h: f32,
}

impl Extents {
    pub fn aspect(&self) -> f32 {
        self.w / self.h
    }
}

pub fn scale_uv(pos: Vec2, dims: Vec2) -> Vec2 {
    let e = Extents {
        w: dims.x,
        h: dims.y,
    };
    Vec2::new(
        pos.x / e.w.max(1.0) * UV_SCALE,
        pos.y / e.h.max(1.0) * UV_SCALE,
    )
}
