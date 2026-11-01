//! Punctum: 3D audio visualizer in a single file.
//! A ring of 64 pillars (one per frequency band) around a bass-pulsing core,
//! raymarched on the GPU. Shader and host code are both plain Rust.
//!
//! Run with: cargo run -p shader-app-3d
mod audio;

use glam::{UVec2, UVec3, Vec2, Vec3};
use punctum::gpu_kernel;
use punctum_common::audio::Spectrum;
use punctum_common::frame::Frame;
use punctum_common::helpers::{tonemap, vignette};
use punctum_common::hud;
use punctum_common::math::{
    Camera, NEON, ShaderParams, TAU, march, normal, sd_box, sd_sphere, smin,
};
use spirv_std::image::StorageImage2d;
use spirv_std::num_traits::Float;

const PILLAR_COUNT: f32 = 64.0;
const RING_RADIUS: f32 = 2.2;
const FLOOR_Y: f32 = -0.9;

// ---------------------------------------------------------------- shader ---

#[inline(always)]
fn map(p: Vec3, audio: &Spectrum) -> f32 {
    // core pulses with the bass
    let core = sd_sphere(p, 0.55 + audio.bass() * 0.4);

    // 64 pillars around the core, one per frequency band
    let r = Vec2::new(p.x, p.z).length();
    let a = Float::atan2(p.z, p.x) / TAU + 0.5;
    let fi = a * PILLAR_COUNT;
    let idx = (fi as u32).min(63);
    let h = 0.05 + audio.band(idx) * 0.7; // half-height
    let tang = (fi - idx as f32 - 0.5) * TAU * r / PILLAR_COUNT;
    let q = Vec3::new(r - RING_RADIUS, p.y - FLOOR_Y - h, tang);
    let pillar = sd_box(q, Vec3::new(0.06, h, 0.07)) * 0.7;

    smin(core, pillar, 0.15)
}

#[inline(always)]
fn scene(f: &Frame, audio: &Spectrum) -> Vec3 {
    // orbiting camera, mouse X steers it
    let cam = Camera::orbit(Vec3::ZERO, 5.5, f.t * 0.15 + f.mouse.x * 1.5, 2.2);
    let ray = cam.ray(f.uv);
    let hit = march(ray.origin, ray.dir, 20.0, 96, 0.002, |p| map(p, audio));

    let mut col = Vec3::new(0.02, 0.02, 0.05);
    if hit.hit {
        let p = ray.origin + ray.dir * hit.t;
        let n = normal(p, |q| map(q, audio));
        let light = Vec3::new(0.5, 0.8, 0.3).normalize();
        let diff = n.dot(light).max(0.0);
        let fres = 1.0 - n.dot(-ray.dir).max(0.0);
        let rim = fres * fres * fres;
        let base = NEON.at(p.y * 0.3 + f.t * 0.1);
        col = base * (0.15 + diff) + base * rim;
    }

    // soft glow from near-misses
    col += NEON.at(f.t * 0.1) * 0.02 / (0.02 + hit.min_d);

    vignette(tonemap(col), f.uv01, 1.2)
}

#[gpu_kernel(16, 16, 1)]
pub fn visualizer(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &StorageImage2d,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] params: &ShaderParams,
    #[spirv(uniform, descriptor_set = 0, binding = 2)] spectrum: &Spectrum,
) {
    let dims: UVec2 = output.query_size();
    if id.x >= dims.x || id.y >= dims.y {
        return;
    }
    let f = Frame::new(id, dims, params);
    let mut cv = f.canvas(scene(&f, spectrum));
    hud::stats(&mut cv, &f, spectrum.level());
    f.write(output, cv);
}

fn main() -> anyhow::Result<()> {
    let mut params = ShaderParams::default();
    let (mut fps, mut ms) = (60.0f32, 16.0f32);
    let mut spectrum = Spectrum::default();
    let mut audio = crate::audio::Audio::new()?;

    let spv: &[u8] = punctum::include_spv!();

    punctum::builder()
        .with_title("Punctum: 3D Audio Visualizer")
        .with_size(1280, 720)
        .with_spv(spv)
        .with_kernel(visualizer)
        .with_uniform(1, &params)
        .with_uniform(2, &spectrum)
        .with_vsync(true)
        .on_update(move |ctx| {
            let dt = ctx.delta_time.max(1e-5);
            fps += (1.0 / dt - fps) * 0.05;
            ms += (dt * 1000.0 - ms) * 0.05;

            spectrum.bands = audio.update();
            params = ShaderParams {
                time: ctx.total_time,
                delta_time: ctx.delta_time,
                fps,
                frame_ms: ms,
                mouse_x: ctx.mouse[0],
                mouse_y: ctx.mouse[1],
                mouse_down: if ctx.mouse_down { 1.0 } else { 0.0 },
                _pad: 0.0,
            };
            ctx.update_buffer(1, &params);
            ctx.update_buffer(2, &spectrum);
        })
        .run()
}
