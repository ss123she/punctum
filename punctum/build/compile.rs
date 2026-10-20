use std::path::PathBuf;

use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::Capability;
use cargo_gpu_install::spirv_builder::SpirvMetadata;

pub(crate) fn get_workspace_target_dir() -> PathBuf {
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target);
    }

    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let out_path = PathBuf::from(out_dir);

        if let Some(target_dir) = out_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
        {
            return target_dir.to_path_buf();
        }
    }
    PathBuf::from("target")
}

pub(crate) fn compile_shader_crate(
    crate_path: &PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let backend = Install::from_shader_crate(crate_path.clone()).run()?;
    let mut builder = backend.to_spirv_builder(crate_path, "spirv-unknown-vulkan1.2");
    builder.build_script.defaults = true;
    builder.spirv_metadata = SpirvMetadata::Full;

    let spv_result = builder
        .capability(Capability::ImageQuery)
        .capability(Capability::StorageImageWriteWithoutFormat)
        .build()?;

    Ok(spv_result.module.unwrap_single().to_path_buf())
}
