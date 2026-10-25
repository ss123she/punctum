use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::resolve::Resolved;
use crate::rewrite::{emit_item, strip_gpu_kernel_from_item, transform_kernel};
use crate::scan::CrateSource;

pub(crate) fn generate_lib_rs(source: &CrateSource, resolved: &Resolved) -> String {
    let mut out = String::new();
    out.push_str("#![no_std]\n");
    out.push_str(
        "#![allow(unused_imports, unused_variables, dead_code, unused_qualifications)]\n\n",
    );

    for line in &resolved.use_lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');

    let mut emitted: BTreeSet<String> = BTreeSet::new();

    for &index in &resolved.items {
        let entry = &source.items[index];
        let mut item = entry.item.clone();
        if entry.is_kernel {
            if let syn::Item::Fn(f) = &mut item {
                *f = transform_kernel(f);
            }
        } else {
            strip_gpu_kernel_from_item(&mut item);
        }

        let text = emit_item(&mut item, source);
        if emitted.insert(text.clone()) {
            out.push_str(&text);
            out.push_str("\n\n");
        }
    }

    for &index in &resolved.impls {
        let mut item = syn::Item::Impl(source.impls[index].item.clone());
        let text = emit_item(&mut item, source);
        if emitted.insert(text.clone()) {
            out.push_str(&text);
            out.push_str("\n\n");
        }
    }

    out
}

pub(crate) fn load_lock_versions() -> HashMap<String, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();

    let candidates = [
        manifest_dir.join("../Cargo.lock"),
        manifest_dir.join("../../Cargo.lock"),
        manifest_dir.join("Cargo.lock"),
    ];

    let mut versions: HashMap<String, String> = HashMap::new();
    for candidate in candidates {
        let Ok(text) = fs::read_to_string(&candidate) else {
            continue;
        };

        let mut in_package = false;
        let mut current: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_package = trimmed == "[[package]]";
                current = None;
            } else if in_package {
                if let Some(rest) = trimmed.strip_prefix("name = ") {
                    if current.is_none() {
                        current = Some(unquote(rest));
                    }
                } else if let Some(rest) = trimmed.strip_prefix("version = ")
                    && let Some(name) = current.take()
                {
                    versions.entry(name).or_insert_with(|| unquote(rest));
                }
            }
        }
    }
    versions
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

pub(crate) fn load_direct_dependencies() -> HashSet<String> {
    let Ok(text) = fs::read_to_string("Cargo.toml") else {
        return HashSet::new();
    };

    let mut deps: HashSet<String> = HashSet::new();
    let mut in_dependencies = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dependencies = trimmed == "[dependencies]";
            if let Some(rest) = trimmed.strip_prefix("[dependencies.")
                && let Some(name) = rest.strip_suffix(']')
            {
                deps.insert(unquote(name).replace('-', "_"));
            }

            continue;
        }
        if !in_dependencies || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                deps.insert(key.replace('-', "_"));
            }
        }
    }
    deps
}

pub(crate) fn load_path_dependencies() -> HashMap<String, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let Ok(text) = fs::read_to_string(manifest_dir.join("Cargo.toml")) else {
        return HashMap::new();
    };

    let mut out = HashMap::new();
    let mut in_deps = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps || t.starts_with('#') {
            continue;
        }
        let Some((key, rest)) = t.split_once('=') else {
            continue;
        };
        let Some(p) = rest.split("path").nth(1) else {
            continue;
        };
        let Some(path) = p.split('"').nth(1) else {
            continue;
        };

        if let Ok(abs) = manifest_dir.join(path).canonicalize() {
            let s = abs.to_string_lossy().replace('\\', "/");
            let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
            out.insert(key.trim().to_string(), s);
        }
    }
    out
}

pub(crate) fn generate_cargo_toml(
    extern_crates: &BTreeSet<String>,
    lock_versions: &HashMap<String, String>,
    path_deps: &HashMap<String, String>,
) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let spirv_version = lock_versions
        .get("spirv-std")
        .cloned()
        .unwrap_or_else(|| "0.11".to_string());

    let mut deps = format!("spirv-std = \"{spirv_version}\"\n");
    for name in extern_crates {
        let package = name.replace('_', "-");
        if let Some(path) = path_deps.get(&package) {
            deps.push_str(&format!(
                "{package} = {{ path = \"{path}\", default-features = false }}\n"
            ));
            continue;
        }
        match lock_versions.get(&package) {
            Some(version) => deps.push_str(&format!(
                "{name} = {{ version = \"{version}\", default-features = false }}\n"
            )),
            None => warnings.push(format!(
                "shader code uses crate '{name}' but it is not in the dependency graph; add it to the [dependencies] of the package being bundled"
            )),
        }
    }

    let cargo_toml = format!(
        "# Generated by punctum's bundler - do not edit.\n\
         [package]\n\
         name = \"punctum-generated-shaders\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         \n\
         [lib]\n\
         crate-type = [\"lib\"]\n\
         \n\
         [dependencies]\n\
         {deps}\n\
         [workspace]\n"
    );

    (cargo_toml, warnings)
}

pub(crate) fn write_if_changed(
    path: &Path,
    content: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let changed = match fs::read_to_string(path) {
        Ok(old) => old != content,
        Err(_) => true,
    };

    if changed {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    }

    Ok(changed)
}
