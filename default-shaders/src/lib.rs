#![no_std]

use spirv_std::glam::{UVec2, UVec3, vec2, vec4};
use spirv_std::image::StorageImage2d;
use spirv_std::spirv;

#[spirv(compute(threads(16, 16, 1), entry_point_name = "main"))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &StorageImage2d,
) {
    let coords = id.truncate();
    let dims: UVec2 = output.query_size();

    if coords.x < dims.x && coords.y < dims.y {
        let uv = vec2(
            coords.x as f32 / dims.x as f32,
            coords.y as f32 / dims.y as f32,
        );
        let color = vec4(uv.x, uv.y, 0.5, 1.0);

        unsafe {
            output.write(coords, color);
        }
    }
}
