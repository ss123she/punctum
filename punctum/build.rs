use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::Capability;
use cargo_gpu_install::spirv_builder::SpirvMetadata;
use std::path::Path;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");

    let mut found_kernels = Vec::new();
    scan_directory("src", &mut found_kernels)?;
    scan_directory("examples", &mut found_kernels)?;

    for kernel in &found_kernels {
        println!("cargo:warning=Found kernel: {}", kernel);
    }

    let shader_crate = PathBuf::from("../default-shaders");
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

fn scan_directory(dir: &str, found: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(dir).exists() {
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            let content = std::fs::read_to_string(entry.path())?;
            if let Ok(file) = syn::parse_file(&content) {
                for item in file.items {
                    if let syn::Item::Fn(func) = item {
                        let is_kernel = func.attrs.iter().any(|attr| {
                            attr.path().is_ident("gpu_kernel")
                                || attr
                                    .path()
                                    .segments
                                    .last()
                                    .is_some_and(|s| s.ident == "gpu_kernel")
                        });

                        if is_kernel {
                            found.push(func.sig.ident.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
