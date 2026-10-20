mod math;

use math::{Extents, GRID_TINT, scale_uv};
use punctum::gpu_kernel;

#[gpu_kernel(8, 8, 1)]
pub fn tint(
    #[spirv(global_invocation_id)] id: glam::UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &spirv_std::image::StorageImage2d,
) {
    let coords = id.truncate();
    let dims: glam::UVec2 = output.query_size();

    if coords.x < dims.x && coords.y < dims.y {
        let pos = glam::vec2(coords.x as f32, coords.y as f32);
        let uv = scale_uv(pos, glam::vec2(dims.x as f32, dims.y as f32));
        let color = glam::vec4(
            uv.x,
            uv.y,
            GRID_TINT
                * Extents {
                    w: dims.x as f32,
                    h: dims.y as f32,
                }
                .aspect(),
            1.0,
        );

        unsafe {
            output.write(coords, color);
        }
    }
}

fn main() {
    let spv: &[u8] = include_bytes!(env!("PUNCTUM_SPV"));
    println!(
        "kernel '{}' ({}x{}x{}), bundled spv: {} bytes",
        tint.name,
        tint.workgroup_size[0],
        tint.workgroup_size[1],
        tint.workgroup_size[2],
        spv.len()
    );
    punctum::builder()
        .with_title("Punctum Engine!")
        .with_size(1280, 720)
        .with_spv(include_bytes!(env!("PUNCTUM_SPV")))
        .with_kernel(tint)
        .run()
        .expect("Error while building app");
}
