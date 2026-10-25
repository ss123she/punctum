use glam::{Vec2, Vec3, Vec4};
use spirv_std::num_traits::Float;

pub use core::f32::consts::{PI, TAU};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct ShaderParams {
    pub time: f32,
    pub delta_time: f32,
    pub fps: f32,
    pub frame_ms: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_down: f32,
    pub _pad: f32,
}

pub trait Mix {
    fn mix(self, to: Self, t: f32) -> Self;
}
impl Mix for f32 {
    fn mix(self, b: f32, t: f32) -> f32 {
        self + (b - self) * t
    }
}
impl Mix for Vec2 {
    fn mix(self, b: Vec2, t: f32) -> Vec2 {
        self + (b - self) * t
    }
}
impl Mix for Vec3 {
    fn mix(self, b: Vec3, t: f32) -> Vec3 {
        self + (b - self) * t
    }
}
impl Mix for Vec4 {
    fn mix(self, b: Vec4, t: f32) -> Vec4 {
        self + (b - self) * t
    }
}

pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

pub fn fract(x: f32) -> f32 {
    x - Float::floor(x)
}

pub fn step(edge: f32, x: f32) -> f32 {
    if x < edge { 0.0 } else { 1.0 }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = saturate((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

pub fn remap(x: f32, a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    b0 + (x - a0) / (a1 - a0) * (b1 - b0)
}

pub fn rotate(p: Vec2, angle: f32) -> Vec2 {
    let s = Float::sin(angle);
    let c = Float::cos(angle);
    Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c)
}

#[derive(Copy, Clone)]
pub struct Palette {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub d: Vec3,
}

impl Palette {
    pub const fn new(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Self {
        Self { a, b, c, d }
    }

    pub fn at(&self, t: f32) -> Vec3 {
        let x = (self.c * t + self.d) * TAU;
        self.a + self.b * Vec3::new(Float::cos(x.x), Float::cos(x.y), Float::cos(x.z))
    }
}

pub const NEON: Palette = Palette::new(
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(0.263, 0.416, 0.557),
);

pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

impl Camera {
    pub fn new(pos: Vec3, target: Vec3) -> Self {
        Self {
            pos,
            target,
            fov: 1.0,
        }
    }

    pub fn orbit(center: Vec3, radius: f32, angle: f32, height: f32) -> Self {
        let pos = center
            + Vec3::new(
                Float::sin(angle) * radius,
                height,
                Float::cos(angle) * radius,
            );
        Self {
            pos,
            target: center,
            fov: 1.0,
        }
    }

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    pub fn ray(&self, uv: Vec2) -> Ray {
        let fwd = (self.target - self.pos).normalize();
        let right = Vec3::Y.cross(fwd).normalize();
        let up = fwd.cross(right);
        let focal = 1.0 / Float::tan(self.fov * 0.5);
        Ray {
            origin: self.pos,
            dir: (right * uv.x + up * uv.y + fwd * focal).normalize(),
        }
    }
}

pub fn sd_sphere(p: Vec3, r: f32) -> f32 {
    p.length() - r
}

pub fn sd_plane(p: Vec3, height: f32) -> f32 {
    p.y - height
}

pub fn sd_box(p: Vec3, half: Vec3) -> f32 {
    let q = p.abs() - half;
    q.max(Vec3::ZERO).length() + q.x.max(q.y.max(q.z)).min(0.0)
}

pub fn sd_torus(p: Vec3, major: f32, minor: f32) -> f32 {
    let q = Vec2::new(Vec2::new(p.x, p.z).length() - major, p.y);
    q.length() - minor
}

pub fn sd_capsule(p: Vec3, a: Vec3, b: Vec3, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = saturate(pa.dot(ba) / ba.dot(ba));
    (pa - ba * h).length() - r
}

pub fn union(a: f32, b: f32) -> f32 {
    a.min(b)
}

pub fn intersect(a: f32, b: f32) -> f32 {
    a.max(b)
}

pub fn subtract(a: f32, b: f32) -> f32 {
    a.max(-b)
}

pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = saturate(0.5 + 0.5 * (b - a) / k);
    b * (1.0 - h) + a * h - k * h * (1.0 - h)
}

pub fn smax(a: f32, b: f32, k: f32) -> f32 {
    -smin(-a, -b, k)
}

pub fn normal(p: Vec3, f: impl Fn(Vec3) -> f32) -> Vec3 {
    let e = 0.001;
    let k1 = Vec3::new(1.0, -1.0, -1.0);
    let k2 = Vec3::new(-1.0, -1.0, 1.0);
    let k3 = Vec3::new(-1.0, 1.0, -1.0);
    let k4 = Vec3::new(1.0, 1.0, 1.0);
    (k1 * f(p + k1 * e) + k2 * f(p + k2 * e) + k3 * f(p + k3 * e) + k4 * f(p + k4 * e)).normalize()
}

pub struct Hit {
    pub t: f32,
    pub hit: bool,
    pub min_d: f32,
    pub steps: u32,
}

pub fn march(
    ro: Vec3,
    rd: Vec3,
    t_max: f32,
    max_steps: u32,
    eps: f32,
    f: impl Fn(Vec3) -> f32,
) -> Hit {
    let mut t = 0.0;
    let mut min_d = f32::MAX;
    let mut i = 0;
    while i < max_steps && t < t_max {
        let d = f(ro + rd * t);
        min_d = min_d.min(d.abs());
        if d < eps {
            return Hit {
                t,
                hit: true,
                min_d,
                steps: i,
            };
        }
        t += d;
        i += 1;
    }
    Hit {
        t,
        hit: false,
        min_d,
        steps: i,
    }
}
