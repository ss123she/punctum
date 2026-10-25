use core::f32;

use glam::{Vec2, Vec3, Vec4};

pub const PI: f32 = f32::consts::PI;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ShaderParams {
    pub time: f32,
    pub delta_time: f32,
    pub width: f32,
    pub height: f32,
    pub fps: f32,
    pub frame_ms: f32,
    pub level: f32,
    pub _pad: f32,
}

pub struct CameraRay {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl CameraRay {
    pub fn create(uv: Vec2) -> Self {
        let origin = Vec3::new(0.0, 0.0, -3.0);
        let dir = Vec3::new(uv.x, uv.y, 1.0).normalize();
        Self { origin, dir }
    }
}

pub fn rotate_2d(p: Vec2, angle: f32) -> Vec2 {
    let s = spirv_sin(angle);
    let c = spirv_cos(angle);
    Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c)
}

pub fn spirv_sin(x: f32) -> f32 {
    let mut v = x % (2.0 * PI);
    if v > PI {
        v -= 2.0 * PI;
    }
    if v < -PI {
        v += 2.0 * PI;
    }

    if v > PI * 0.5 {
        v = PI - v;
    } else if v < -PI * 0.5 {
        v = -PI - v;
    }

    let v2 = v * v;
    v * (1.0 - v2 / 6.0 * (1.0 - v2 / 20.0 * (1.0 - v2 / 42.0)))
}

pub fn spirv_cos(x: f32) -> f32 {
    spirv_sin(x + PI * 0.5)
}

pub fn sd_torus(p: Vec3, tx: f32, ty: f32) -> f32 {
    let q = Vec2::new(Vec2::new(p.x, p.z).length() - tx, p.y);
    q.length() - ty
}

pub fn cosine_palette(t: f32) -> Vec4 {
    let a = Vec3::new(0.5, 0.5, 0.5);
    let b = Vec3::new(0.5, 0.5, 0.5);
    let c = Vec3::new(1.0, 1.0, 1.0);
    let d = Vec3::new(0.263, 0.416, 0.557);

    let color = a + b * Vec3::new(
        spirv_cos(2.0 * PI * (c.x * t + d.x)),
        spirv_cos(2.0 * PI * (c.y * t + d.y)),
        spirv_cos(2.0 * PI * (c.z * t + d.z)),
    );

    Vec4::new(color.x, color.y, color.z, 1.0)
}

pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

pub fn blob_pos(t: f32, i: f32) -> Vec3 {
    let ph = i * 1.2566;
    Vec3::new(
        spirv_sin(t * 0.7 + ph * 2.0) * 1.3,
        spirv_sin(t * 0.9 + ph * 3.0) * 0.8,
        spirv_cos(t * 0.6 + ph) * 1.3,
    )
}

pub fn blob_positions(t: f32) -> [Vec3; 5] {
    [
        blob_pos(t, 0.0),
        blob_pos(t, 1.0),
        blob_pos(t, 2.0),
        blob_pos(t, 3.0),
        blob_pos(t, 4.0),
    ]
}

pub fn blobs_sdf(p: Vec3, b: [Vec3; 5]) -> f32 {
    let r = p.length();
    if r > 4.0 {
        return r - 2.55;
    }
    let mut d = (p - b[0]).length() - 0.55;
    let mut i = 1;
    while i < 5 {
        let s = (p - b[i]).length() - 0.45;
        d = smin(d, s, 0.6);
        i += 1;
    }
    d
}

pub fn floor_sdf(p: Vec3) -> f32 {
    p.y + 1.6
}

pub fn scene(p: Vec3, b: [Vec3; 5]) -> f32 {
    blobs_sdf(p, b).min(floor_sdf(p))
}

pub fn calc_normal(p: Vec3, b: [Vec3; 5]) -> Vec3 {
    let e = 0.001;
    let k1 = Vec3::new(1.0, -1.0, -1.0);
    let k2 = Vec3::new(-1.0, -1.0, 1.0);
    let k3 = Vec3::new(-1.0, 1.0, -1.0);
    let k4 = Vec3::new(1.0, 1.0, 1.0);
    (k1 * scene(p + k1 * e, b)
        + k2 * scene(p + k2 * e, b)
        + k3 * scene(p + k3 * e, b)
        + k4 * scene(p + k4 * e, b))
    .normalize()
}

pub fn sky(dir: Vec3, t: f32, light: Vec3) -> Vec3 {
    let y = dir.y * 0.5 + 0.5;
    let base = cosine_palette(y * 0.6 + t * 0.05).truncate() * (0.15 + y * 0.6);
    let s = dir.dot(light).max(0.0);
    let s2 = s * s;
    let s4 = s2 * s2;
    let s8 = s4 * s4;
    base + Vec3::splat(s8 * s8 * 2.0)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Spectrum {
    pub bands: [Vec4; 16],
}
