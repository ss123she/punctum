mod compile;
mod emit;
mod resolve;
mod rewrite;
mod scan;
mod uses;

use std::fs;

use compile::{compile_shader_crate, get_workspace_target_dir};
use emit::{
    generate_cargo_toml, generate_lib_rs, load_direct_dependencies, load_lock_versions,
    write_if_changed,
};
use resolve::resolve_all;
use scan::{collect_source, scan_directory};

use crate::emit::load_path_dependencies;

pub struct Config {
    pub scan_dirs: Vec<String>,
    pub shadow_subdir: String,
    pub env_var: String,
    pub skip_roots: Vec<String>,
}

impl Config {
    pub fn for_current_package() -> Self {
        let pkg = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "app".to_string());
        Self {
            scan_dirs: vec!["src".to_string()],
            shadow_subdir: format!("punctum_shaders/{pkg}"),
            env_var: "PUNCTUM_SPV".to_string(),
            skip_roots: vec!["punctum".to_string()],
        }
    }
}

pub fn run(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    for dir in &config.scan_dirs {
        println!("cargo:rerun-if-changed={dir}");
    }

    let mut files: Vec<(String, syn::File)> = Vec::new();
    for dir in &config.scan_dirs {
        scan_directory(dir, &mut files)?;
    }

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
    let resolved = resolve_all(&source, &kernel_indices, &direct_deps, &config.skip_roots);
    for warning in &resolved.warnings {
        println!("cargo:warning={warning}");
    }

    let shadow_crate_dir = get_workspace_target_dir().join(&config.shadow_subdir);
    let cached_spv = shadow_crate_dir.join("current_kernels.spv");

    let lib_rs = generate_lib_rs(&source, &resolved);
    let (cargo_toml, toml_warnings) = generate_cargo_toml(
        &resolved.extern_crates,
        &lock_versions,
        &load_path_dependencies(),
    );
    for warning in &toml_warnings {
        println!("cargo:warning={warning}");
    }

    let lib_changed = write_if_changed(&shadow_crate_dir.join("src/lib.rs"), &lib_rs)?;
    let toml_changed = write_if_changed(&shadow_crate_dir.join("Cargo.toml"), &cargo_toml)?;

    if !lib_changed && !toml_changed && cached_spv.exists() {
        println!(
            "cargo:rustc-env={}={}",
            config.env_var,
            cached_spv.display()
        );
        return Ok(());
    }

    match compile_shader_crate(&shadow_crate_dir) {
        Ok(spv_path) => {
            if let Err(e) = fs::copy(&spv_path, &cached_spv) {
                println!(
                    "cargo:warning=Failed to cache SPV at {}: {e}",
                    cached_spv.display()
                );
            }
            println!("cargo:rustc-env={}={}", config.env_var, spv_path.display());
        }
        Err(e) => {
            let _ = fs::remove_file(&cached_spv);
            println!("cargo:warning=Error in shader code: {e}");

            let pkg = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
            let target = get_workspace_target_dir();

            let err_file = target.join(format!("punctum_error_{pkg}.txt"));
            let _ = fs::write(&err_file, e.to_string());
            println!(
                "cargo:rustc-env={}_ERROR={}",
                config.env_var,
                err_file.display()
            );

            let dummy_spv = target.join(format!("dummy_{pkg}.spv"));
            let _ = fs::write(&dummy_spv, [0u8; 4]);
            println!("cargo:rustc-env={}={}", config.env_var, dummy_spv.display());
        }
    }

    Ok(())
}
