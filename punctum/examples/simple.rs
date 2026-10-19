use punctum::gpu_kernel;

#[gpu_kernel(16, 16, 1)]
pub fn gradient(
    #[spirv(global_invocation_id)] id: spirv_std::glam::UVec3,
    #[spirv(descriptor_set = 0, binding = 0)] output: &spirv_std::image::StorageImage2d,
) {
    let coords = id.truncate();
    let dims: spirv_std::glam::UVec2 = output.query_size();

    if coords.x < dims.x && coords.y < dims.y {
        let uv = spirv_std::glam::vec2(
            coords.x as f32 / dims.x as f32,
            coords.y as f32 / dims.y as f32,
        );
        let color = spirv_std::glam::vec4(uv.x, uv.y, 0.5, 1.0);

        unsafe {
            output.write(coords, color);
        }
    }
}

fn main() -> anyhow::Result<()> {
    punctum::builder()
        .with_title("Awesome Engine")
        .with_kernel(gradient)
        .with_size(900, 800)
        .run()
}
