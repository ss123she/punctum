use cargo_gpu_install::install::Install;
use cargo_gpu_install::spirv_builder::Capability;
use cargo_gpu_install::spirv_builder::SpirvMetadata;
use quote::quote;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use syn::LitInt;
use syn::Meta;
use syn::Token;
use syn::punctuated::Punctuated;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=examples");

    let mut found_kernels: Vec<syn::ItemFn> = Vec::new();
    scan_directory("src", &mut found_kernels)?;
    scan_directory("examples", &mut found_kernels)?;

    for kernel in &found_kernels {
        println!(
            "cargo:warning=Found kernel: {}",
            kernel.sig.ident.to_string()
        );
    }

    if !found_kernels.is_empty() {
        let shadow_crate_dir = get_workspace_target_dir().join("punctum_shaders");
        let cached_spv = shadow_crate_dir.join("current_kernels.spv");

        let code_changed = generate_shadow_crate(&shadow_crate_dir, &found_kernels)?;

        if !code_changed && cached_spv.exists() {
            println!("cargo:rustc-env=COMPUTE_SPV={}", cached_spv.display());
            return Ok(());
        }

        match compile_shader_crate(&shadow_crate_dir) {
            Ok(spv_path) => {
                println!("cargo:rustc-env=COMPUTE_SPV={}", spv_path.display());
            }
            Err(e) => {
                println!("cargo:warning=Error in shader code: {}", e);

                let dummy_spv = get_workspace_target_dir().join("dummy.spv");
                let _ = std::fs::write(&dummy_spv, &[0u8; 4]);
                println!("cargo:rustc-env=COMPUTE_SPV={}", dummy_spv.display());
            }
        }
    }

    Ok(())
}

fn scan_directory(
    dir: &str,
    found: &mut Vec<syn::ItemFn>,
) -> Result<(), Box<dyn std::error::Error>> {
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
                            found.push(func);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn get_workspace_target_dir() -> PathBuf {
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

fn generate_shadow_crate(
    dir: &Path,
    kernels: &[syn::ItemFn],
) -> Result<bool, Box<dyn std::error::Error>> {
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let cargo_toml = r#"
[package]
name = "punctum-generated-shaders"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib"]

[dependencies]
spirv-std = "0.11"

[workspace]
"#;
    fs::write(dir.join("Cargo.toml"), cargo_toml)?;

    let mut transformed_kernels = Vec::new();
    for func in kernels {
        let mut f = func.clone();
        let (tx, ty, tz) = parse_kernel_threads(&f.attrs);
        f.attrs.retain(|attr| {
            !attr.path().is_ident("gpu_kernel")
                && !attr
                    .path()
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "gpu_kernel")
        });

        f.attrs
            .push(syn::parse_quote!(#[spirv(compute(threads(#tx, #ty, #tz)))]));

        transformed_kernels.push(f);
    }

    let lib_rs = quote! {
        #![no_std]
        #![allow(unused_imports)]
        use spirv_std::spirv;
        use spirv_std::glam::{self, UVec2, UVec3, Vec2, Vec4, vec2, vec4};
        use spirv_std::image::StorageImage2d;

        #(#transformed_kernels)*
    };

    let new_code = lib_rs.to_string();
    let lib_path = src_dir.join("lib.rs");

    let code_changed = if lib_path.exists() {
        let old_code = fs::read_to_string(&lib_path)?;
        old_code != new_code
    } else {
        true
    };

    if code_changed {
        fs::write(&lib_path, new_code)?;
    }

    Ok(code_changed)
}

fn compile_shader_crate(crate_path: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
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

fn parse_kernel_threads(attrs: &[syn::Attribute]) -> (u32, u32, u32) {
    for attr in attrs {
        let is_gpu = attr.path().is_ident("gpu_kernel")
            || attr
                .path()
                .segments
                .last()
                .is_some_and(|s| s.ident == "gpu_kernel");

        if is_gpu {
            if let Meta::List(list) = &attr.meta {
                let parser = Punctuated::<LitInt, Token![,]>::parse_terminated;
                if let Ok(args) = list.parse_args_with(parser) {
                    let mut dims = [1u32; 3];
                    for (i, arg) in args.iter().enumerate().take(3) {
                        if let Ok(v) = arg.base10_parse() {
                            dims[i] = v;
                        }
                    }
                    return (dims[0], dims[1], dims[2]);
                }
            }
            return (16, 16, 1);
        }
    }
    (16, 16, 1)
}
