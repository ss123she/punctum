#[path = "build/compile.rs"]
mod compile;
#[path = "build/emit.rs"]
mod emit;
#[path = "build/resolve.rs"]
mod resolve;
#[path = "build/rewrite.rs"]
mod rewrite;
#[path = "build/scan.rs"]
mod scan;
#[path = "build/uses.rs"]
mod uses;

use std::fs;

use compile::{compile_shader_crate, get_workspace_target_dir};
use emit::{
    generate_cargo_toml, generate_lib_rs, load_direct_dependencies, load_lock_versions,
    write_if_changed,
};
use resolve::resolve_all;
use scan::{collect_source, scan_directory};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");

    let mut files: Vec<(String, syn::File)> = Vec::new();
    scan_directory("src", &mut files)?;
    scan_directory("examples", &mut files)?;

    let source = collect_source(files);
    let kernel_indices: Vec<usize> = source
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_kernel)
        .map(|(index, _)| index)
        .collect();

    for kernel in &kernel_indices {
        println!("cargo:warning=Found kernel: {}", source.items[*kernel].name);
    }

    if kernel_indices.is_empty() {
        return Ok(());
    }

    let lock_versions = load_lock_versions();
    let direct_deps = load_direct_dependencies();
    let resolved = resolve_all(&source, &kernel_indices, &direct_deps);
    for warning in &resolved.warnings {
        println!("cargo:warning={warning}");
    }

    let shadow_crate_dir = get_workspace_target_dir().join("punctum_shaders");
    let cached_spv = shadow_crate_dir.join("current_kernels.spv");

    let lib_rs = generate_lib_rs(&source, &resolved);
    let (cargo_toml, toml_warnings) = generate_cargo_toml(&resolved.extern_crates, &lock_versions);
    for warning in &toml_warnings {
        println!("cargo:warning={warning}");
    }

    let lib_changed = write_if_changed(&shadow_crate_dir.join("src/lib.rs"), &lib_rs)?;
    let toml_changed = write_if_changed(&shadow_crate_dir.join("Cargo.toml"), &cargo_toml)?;

    if !lib_changed && !toml_changed && cached_spv.exists() {
        println!("cargo:rustc-env=COMPUTE_SPV={}", cached_spv.display());
        return Ok(());
    }

    match compile_shader_crate(&shadow_crate_dir) {
        Ok(spv_path) => {
            let _ = fs::copy(&spv_path, &cached_spv);
            println!("cargo:rustc-env=COMPUTE_SPV={}", spv_path.display());
        }
        Err(e) => {
            println!("cargo:warning=Error in shader code: {e}");

            let dummy_spv = get_workspace_target_dir().join("dummy.spv");
            let _ = fs::write(&dummy_spv, [0u8; 4]);
            println!("cargo:rustc-env=COMPUTE_SPV={}", dummy_spv.display());
        }
    }

    Ok(())
}
