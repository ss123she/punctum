use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::Capability;
use cargo_gpu_install::spirv_builder::SpirvMetadata;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shader_crate = PathBuf::from("./default-shaders");

    let backend = Install::from_shader_crate(shader_crate.clone()).run()?;

    let mut builder = backend.to_spirv_builder(shader_crate, "spirv-unknown-vulkan1.2");
    builder.build_script.defaults = true;
    builder.spirv_metadata = SpirvMetadata::Full;

    let spv_result = builder
        .capability(Capability::ImageQuery)
        .capability(Capability::StorageImageWriteWithoutFormat)
        .build()?;
    let path_to_spv = spv_result.module.unwrap_single();

    println!("cargo:rustc-env=COMPUTE_SPV={}", path_to_spv.display());

    Ok(())
}
