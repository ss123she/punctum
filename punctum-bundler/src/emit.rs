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

fn strip_toml_comment(line: &str) -> &str {
    let mut in_quote = false;
    let mut prev_char = '\0';
    for (i, ch) in line.char_indices() {
        if ch == '"' && prev_char != '\\' {
            in_quote = !in_quote;
        } else if ch == '#' && !in_quote {
            return &line[..i];
        }
        prev_char = ch;
    }
    line
}

fn join_multiline_toml(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut open_brackets = 0i32;
    let mut open_braces = 0i32;

    for line in text.lines() {
        let clean = strip_toml_comment(line).trim();
        if clean.is_empty() {
            if current.is_empty() {
                continue;
            }
        }

        let mut in_quote = false;
        let mut prev_char = '\0';
        for ch in clean.chars() {
            if ch == '"' && prev_char != '\\' {
                in_quote = !in_quote;
            } else if !in_quote {
                match ch {
                    '[' => open_brackets += 1,
                    ']' => open_brackets = (open_brackets - 1).max(0),
                    '{' => open_braces += 1,
                    '}' => open_braces = (open_braces - 1).max(0),
                    _ => {}
                }
            }
            prev_char = ch;
        }

        if current.is_empty() {
            current.push_str(clean);
        } else {
            current.push(' ');
            current.push_str(clean);
        }

        if open_brackets == 0 && open_braces == 0 {
            lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(crate) fn load_direct_dependencies() -> HashMap<String, Vec<String>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();
    let manifest_path = if manifest_dir.as_os_str().is_empty() {
        PathBuf::from("Cargo.toml")
    } else {
        manifest_dir.join("Cargo.toml")
    };

    let Ok(text) = fs::read_to_string(&manifest_path) else {
        return HashMap::new();
    };

    let mut map = HashMap::new();
    let mut in_deps = false;
    for line in join_multiline_toml(&text) {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps || t.starts_with('#') {
            continue;
        }

        if let Some((name, rest)) = t.split_once('=') {
            let raw_key = name.trim().to_string();
            let norm_key = raw_key.replace('-', "_");
            let mut features = Vec::new();

            if let Some(pos) = rest.find("features") {
                if let Some(start) = rest[pos..].find('[') {
                    if let Some(end) = rest[pos + start..].find(']') {
                        let inner = &rest[pos + start + 1..pos + start + end];
                        for f in inner.split(',') {
                            let clean = f.trim().trim_matches('"').trim_matches('\'').trim();
                            if !clean.is_empty() {
                                features.push(clean.to_string());
                            }
                        }
                    }
                }
            }
            map.insert(raw_key, features.clone());
            map.insert(norm_key, features);
        }
    }
    map
}

pub(crate) fn load_path_dependencies() -> HashMap<String, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_default();

    let candidates = [
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("../Cargo.toml"),
        manifest_dir.join("../../Cargo.toml"),
    ];

    let mut out = HashMap::new();
    for candidate in candidates {
        let Ok(text) = fs::read_to_string(&candidate) else {
            continue;
        };
        let base_dir = candidate.parent().unwrap_or(&manifest_dir);

        let mut in_deps = false;
        for line in join_multiline_toml(&text) {
            let t = line.trim();
            if t.starts_with('[') {
                in_deps = t == "[dependencies]" || t == "[workspace.dependencies]";
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

            if let Ok(abs) = base_dir.join(path).canonicalize() {
                let s = abs.to_string_lossy().replace('\\', "/");
                let s = s.strip_prefix("//?/").unwrap_or(&s).to_string();
                let clean_key = key.trim().replace('-', "_");
                out.insert(clean_key, s.clone());
                out.insert(key.trim().to_string(), s);
            }
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
    let current_pkg = std::env::var("CARGO_PKG_NAME")
        .unwrap_or_default()
        .replace('-', "_");

    let direct_features = load_direct_dependencies();

    let spirv_version = lock_versions
        .get("spirv-std")
        .cloned()
        .unwrap_or_else(|| "0.11".to_string());

    let mut deps = format!("spirv-std = \"{spirv_version}\"\n");
    for name in extern_crates {
        if name.replace('-', "_") == current_pkg {
            continue;
        }

        let package = name.replace('_', "-");

        let feats = direct_features
            .get(name)
            .or_else(|| direct_features.get(&package));

        let feat_str = match feats {
            Some(list) if !list.is_empty() => {
                let inside = list
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(", features = [{}]", inside)
            }
            _ => String::new(),
        };

        if let Some(path) = path_deps.get(&package) {
            deps.push_str(&format!(
                "{name} = {{ package = \"{package}\", path = \"{path}\", default-features = false{feat_str} }}\n"
            ));
            continue;
        }

        let version = lock_versions
            .get(&package)
            .or_else(|| lock_versions.get(name));

        match version {
            Some(version) => deps.push_str(&format!(
                "{name} = {{ package = \"{package}\", version = \"{version}\", default-features = false{feat_str} }}\n"
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
         [profile.dev]\n\
         debug-assertions = false\n\
         overflow-checks = false\n\
         \n\
         [profile.release]\n\
         opt-level = 3\n\
         debug-assertions = false\n\
         overflow-checks = false\n\
         \n\
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
