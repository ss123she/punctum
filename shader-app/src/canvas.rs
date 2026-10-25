use glam::{UVec2, Vec2, Vec3};

pub struct Canvas {
    pub px: UVec2,
    pub p: Vec2,
    pub col: Vec3,
    pub shadow: f32,
}

impl Canvas {
    pub fn new(coords: UVec2, col: Vec3) -> Self {
        Self {
            px: coords,
            p: Vec2::new(coords.x as f32 + 0.5, coords.y as f32 + 0.5),
            col,
            shadow: 0.6,
        }
    }

    fn blend(&mut self, c: Vec3, a: f32) {
        self.col = self.col + (c - self.col) * a.clamp(0.0, 1.0);
    }

    pub fn fill_sdf(&mut self, d: f32, c: Vec3, alpha: f32) {
        self.blend(c, (0.5 - d).clamp(0.0, 1.0) * alpha);
    }

    pub fn rect(&mut self, pos: Vec2, size: Vec2, c: Vec3, alpha: f32) {
        self.rrect(pos, size, 0.0, c, alpha);
    }

    pub fn rrect(&mut self, pos: Vec2, size: Vec2, r: f32, c: Vec3, alpha: f32) {
        let half = size * 0.5;
        let q = (self.p - pos - half).abs() - (half - Vec2::splat(r));
        let d = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r;
        self.fill_sdf(d, c, alpha);
    }

    pub fn circle(&mut self, center: Vec2, r: f32, c: Vec3, alpha: f32) {
        self.fill_sdf((self.p - center).length() - r, c, alpha);
    }

    pub fn line(&mut self, a: Vec2, b: Vec2, width: f32, c: Vec3, alpha: f32) {
        let pa = self.p - a;
        let ba = b - a;
        let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
        self.fill_sdf((pa - ba * h).length() - width * 0.5, c, alpha);
    }

    fn ink(&mut self, shadow: f32, fill: f32, c: Vec3) {
        self.blend(Vec3::ZERO, shadow * self.shadow);
        self.blend(c, fill);
    }

    pub fn text<const N: usize>(&mut self, o: UVec2, s: u32, c: Vec3, t: &[u32; N]) {
        let sh = crate::text::text_mask(self.px, o + UVec2::splat(s), s, t);
        let fi = crate::text::text_mask(self.px, o, s, t);
        self.ink(sh, fi, c);
    }

    pub fn number(&mut self, o: UVec2, s: u32, c: Vec3, v: u32) {
        let sh = crate::text::number_mask(self.px, o + UVec2::splat(s), s, v);
        let fi = crate::text::number_mask(self.px, o, s, v);
        self.ink(sh, fi, c);
    }

    pub fn fixed(&mut self, o: UVec2, s: u32, c: Vec3, v: f32, decimals: u32) {
        let sh = crate::text::fixed_mask(self.px, o + UVec2::splat(s), s, v, decimals);
        let fi = crate::text::fixed_mask(self.px, o, s, v, decimals);
        self.ink(sh, fi, c);
    }

    pub fn finish(&self) -> Vec3 {
        let h = self.px.x.wrapping_mul(73856093) ^ self.px.y.wrapping_mul(19349663);
        let n = ((h >> 8) & 255) as f32 / 255.0 - 0.5;
        (self.col + Vec3::splat(n / 255.0)).clamp(Vec3::ZERO, Vec3::ONE)
    }
}
