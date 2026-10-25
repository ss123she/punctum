mod audio;

use glam::{UVec2, UVec3, Vec2, Vec3, Vec4};
use punctum::gpu_kernel;
use spirv_std::image::StorageImage2d;

use punctum_common::canvas::Canvas;
use punctum_common::math::{ShaderParams, Spectrum, cosine_palette};
use punctum_common::text;

#[gpu_kernel(16, 16, 1)]
pub fn visualizer(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &StorageImage2d,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] params: &ShaderParams,
    #[spirv(uniform, descriptor_set = 0, binding = 2)] spectrum: &Spectrum,
) {
    let coords = id.truncate();
    let dims: UVec2 = output.query_size();
    if coords.x >= dims.x || coords.y >= dims.y {
        return;
    }

    let t = params.time;

    let p = Vec2::new(
        coords.x as f32 / dims.x as f32,
        coords.y as f32 / dims.y as f32,
    );
    let h_up = 1.0 - p.y;

    let fx = p.x * 64.0;
    let idx = (fx as u32).min(63);
    let local = fx - idx as f32;
    let v4 = spectrum.bands[(idx / 4) as usize];
    let k = idx % 4;
    let v = if k == 0 {
        v4.x
    } else if k == 1 {
        v4.y
    } else if k == 2 {
        v4.z
    } else {
        v4.w
    };

    let height = v * 0.85;

    let edge = ((0.42 - (local - 0.5).abs()) * 25.0).clamp(0.0, 1.0);
    let body = ((height - h_up) * 250.0).clamp(0.0, 1.0) * edge;

    let dy = (h_up - height).max(0.0);
    let glow = edge * v * 0.03 / (0.03 + dy);

    let bar_col = cosine_palette(p.x * 0.8 + t * 0.1).truncate();
    let grad = 0.35 + 0.65 * (h_up / height.max(0.01)).min(1.0);

    let mut col = Vec3::new(0.02, 0.02, 0.05) + Vec3::splat(0.04 * h_up);
    col += bar_col * (body * grad + glow * 0.6);

    let mut cv = Canvas::new(coords, col);

    if coords.x < 200 && coords.y < 80 {
        const L_FPS: [u32; 3] = text::codes("FPS");
        const L_MS: [u32; 2] = text::codes("MS");
        const L_LVL: [u32; 3] = text::codes("LVL");

        let white = Vec3::ONE;
        let dim = Vec3::splat(0.6);
        let accent = cosine_palette(params.time * 0.1).truncate();

        cv.rrect(
            Vec2::new(8.0, 8.0),
            Vec2::new(180.0, 62.0),
            6.0,
            Vec3::ZERO,
            0.45,
        );

        cv.text(UVec2::new(16, 16), 2, dim, &L_FPS);
        cv.fixed(UVec2::new(60, 16), 2, white, params.fps, 1);

        cv.text(UVec2::new(16, 30), 2, dim, &L_MS);
        cv.fixed(UVec2::new(60, 30), 2, white, params.frame_ms, 2);

        cv.text(UVec2::new(16, 44), 2, dim, &L_LVL);
        cv.rect(Vec2::new(60.0, 46.0), Vec2::new(116.0, 6.0), white, 0.15);
        cv.rect(
            Vec2::new(60.0, 46.0),
            Vec2::new(116.0 * params.level.clamp(0.0, 1.0), 6.0),
            accent,
            1.0,
        );
    }

    let col = cv.finish();
    unsafe {
        output.write(coords, Vec4::new(col.x, col.y, col.z, 1.0));
    }
}

fn main() -> anyhow::Result<()> {
    let (width, height) = (1280u32, 720u32);

    let mut params = ShaderParams {
        width: width as f32,
        height: height as f32,
        ..Default::default()
    };
    let (mut fps, mut ms) = (60.0f32, 16.0f32);
    let mut spectrum = Spectrum::default();
    let mut audio = audio::Audio::new()?;

    let spv: &[u8] = punctum::include_spv!();

    punctum::builder()
        .with_title("Punctum: Audio Visualizer")
        .with_size(width, height)
        .with_spv(spv)
        .with_kernel(visualizer)
        .with_uniform(1, &params)
        .with_uniform(2, &spectrum)
        .with_vsync(true)
        .on_update(move |ctx| {
            let dt = ctx.delta_time.max(1e-5);
            fps += (1.0 / dt - fps) * 0.05;
            ms += (dt * 1000.0 - ms) * 0.05;

            let bands = audio.update();
            spectrum.bands = bands;
            params.level = bands.iter().map(|b| b.x + b.y + b.z + b.w).sum::<f32>() / 64.0;

            params.time = ctx.total_time;
            params.delta_time = ctx.delta_time;
            params.fps = fps;
            params.frame_ms = ms;

            ctx.update_buffer(1, &params);
            ctx.update_buffer(2, &spectrum);
        })
        .run()
}
