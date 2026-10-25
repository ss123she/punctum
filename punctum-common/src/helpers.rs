use glam::{UVec2, Vec2, Vec3};
use spirv_std::num_traits::Float;

use crate::math::{Mix, TAU, fract, saturate, smoothstep, step};

pub fn pulse(t: f32, hz: f32) -> f32 {
    0.5 + 0.5 * Float::sin(t * hz * TAU)
}

pub fn saw(t: f32, hz: f32) -> f32 {
    fract(t * hz)
}

pub fn tri(t: f32, hz: f32) -> f32 {
    (fract(t * hz) * 2.0 - 1.0).abs()
}

pub fn square(t: f32, hz: f32, duty: f32) -> f32 {
    step(duty, fract(t * hz))
}

pub fn ease_in(t: f32) -> f32 {
    t * t
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub fn ease_in_out(t: f32) -> f32 {
    smoothstep(0.0, 1.0, t)
}

pub fn hsv(h: f32, s: f32, v: f32) -> Vec3 {
    let s = saturate(s);
    let ch = |n: f32| {
        let k = fract((n + h * 6.0) / 6.0) * 6.0;
        let m = k.min(4.0 - k).min(1.0).max(0.0);
        v - v * s * m
    };
    Vec3::new(ch(5.0), ch(3.0), ch(1.0))
}

pub fn tonemap(c: Vec3) -> Vec3 {
    c / (c + Vec3::ONE)
}

pub fn vignette(c: Vec3, uv01: Vec2, strength: f32) -> Vec3 {
    c * (1.0 - ((uv01 - Vec2::splat(0.5)).length() * strength).min(1.0))
}

pub fn hash(n: u32) -> u32 {
    let x = n.wrapping_mul(747796405).wrapping_add(2891336453);
    let x = ((x >> ((x >> 28) + 4)) ^ x).wrapping_mul(277803737);
    (x >> 22) ^ x
}

pub fn rand(n: u32) -> f32 {
    (hash(n) >> 8) as f32 / 16777216.0
}

pub fn hash2(p: UVec2) -> u32 {
    hash(p.x ^ hash(p.y))
}

pub fn rand2(p: UVec2) -> f32 {
    (hash2(p) >> 8) as f32 / 16777216.0
}

pub fn value_noise(p: Vec2) -> f32 {
    let i = Vec2::new(Float::floor(p.x), Float::floor(p.y));
    let f = p - i;
    let u = f * f * (Vec2::splat(3.0) - 2.0 * f);

    let ix = i.x as i32 as u32;
    let iy = i.y as i32 as u32;

    let a = rand2(UVec2::new(ix, iy));
    let b = rand2(UVec2::new(ix.wrapping_add(1), iy));
    let c = rand2(UVec2::new(ix, iy.wrapping_add(1)));
    let d = rand2(UVec2::new(ix.wrapping_add(1), iy.wrapping_add(1)));

    a.mix(b, u.x).mix(c.mix(d, u.x), u.y)
}

pub fn fbm(p: Vec2, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut q = p;
    let mut i = 0;
    while i < octaves {
        sum += amp * value_noise(q);
        q = q * 2.0;
        amp *= 0.5;
        i += 1;
    }
    sum
}
