mod audio;

use glam::{UVec2, UVec3, Vec3};
use punctum::gpu_kernel;
use punctum_common::audio::Spectrum;
use punctum_common::frame::Frame;
use punctum_common::hud;
use punctum_common::math::{NEON, ShaderParams};
use spirv_std::image::StorageImage2d;

#[inline(always)]
fn scene(f: &Frame, audio: &Spectrum) -> Vec3 {
    let h_up = f.uv01.y;

    let fx = f.uv01.x * 64.0;
    let idx = (fx as u32).min(63);
    let local = fx - idx as f32;
    let v = audio.band(idx);

    let height = v * 0.85;

    let edge = ((0.42 - (local - 0.5).abs()) * 25.0).clamp(0.0, 1.0);
    let body = ((height - h_up) * 250.0).clamp(0.0, 1.0) * edge;

    let dy = (h_up - height).max(0.0);
    let glow = edge * v * 0.03 / (0.03 + dy);

    let bar_col = NEON.at(f.uv01.x * 0.8 + f.t * 0.1);
    let grad = 0.35 + 0.65 * (h_up / height.max(0.01)).min(1.0);

    let mut col = Vec3::new(0.02, 0.02, 0.05) + Vec3::splat(0.04 * h_up);
    col += bar_col * (body * grad + glow * 0.6);

    let m = (f.uv - f.mouse).length();
    let click = if f.mouse_down { 3.0 } else { 1.0 };
    col += NEON.at(f.t * 0.2) * 0.12 * click / (1.0 + 60.0 * m * m);

    col
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
    let mut audio = audio::Audio::new()?;

    let spv: &[u8] = punctum::include_spv!();

    punctum::builder()
        .with_title("Punctum: Audio Visualizer")
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
