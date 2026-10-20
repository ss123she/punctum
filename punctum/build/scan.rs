use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub(crate) const RESERVED_ROOTS: &[&str] = &["core", "std", "alloc", "punctum_macros"];

pub(crate) struct SourceItem {
    pub(crate) name: String,
    pub(crate) file: String,
    pub(crate) is_kernel: bool,
    pub(crate) item: syn::Item,
}

pub(crate) struct SourceImpl {
    pub(crate) file: String,
    pub(crate) item: syn::ItemImpl,
}

pub(crate) struct CrateSource {
    pub(crate) pkg_root: String,
    pub(crate) items: Vec<SourceItem>,
    pub(crate) impls: Vec<SourceImpl>,
    pub(crate) modules: HashMap<String, usize>,
    pub(crate) uses: Vec<(String, syn::ItemUse)>,
}

pub(crate) fn scan_directory(
    dir: &str,
    files: &mut Vec<(String, syn::File)>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(dir).exists() {
        return Ok(());
    }

    for entry in WalkDir::new(dir).sort_by(|a, b| a.path().cmp(b.path())) {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "rs") {
            let content = fs::read_to_string(entry.path())?;
            let path = entry.path().to_string_lossy().replace('\\', "/");
            match syn::parse_file(&content) {
                Ok(file) => files.push((path, file)),
                Err(err) => {
                    println!("cargo:warning=Skipping unparsable shader source {path}: {err}")
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn collect_source(files: Vec<(String, syn::File)>) -> CrateSource {
    let pkg_root = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    let mut source = CrateSource {
        pkg_root,
        items: Vec::new(),
        impls: Vec::new(),
        modules: HashMap::new(),
        uses: Vec::new(),
    };

    for (file, parsed) in &files {
        collect_items(&parsed.items, file, &mut source);
    }
    source
}

fn collect_items(items: &[syn::Item], file: &str, source: &mut CrateSource) {
    for item in items {
        match item {
            syn::Item::Use(use_item) => source.uses.push((file.to_string(), use_item.clone())),
            syn::Item::Impl(imp) => source.impls.push(SourceImpl {
                file: file.to_string(),
                item: imp.clone(),
            }),
            syn::Item::Mod(module) => {
                *source.modules.entry(module.ident.to_string()).or_insert(0) += 1;
                if let Some((_, content)) = &module.content {
                    collect_items(content, file, source);
                }
            }
            _ => {
                if let Some(name) = item_name(item) {
                    let is_kernel = is_kernel_fn(item);
                    source.items.push(SourceItem {
                        name,
                        file: file.to_string(),
                        is_kernel,
                        item: item.clone(),
                    });
                }
            }
        }
    }
}

fn item_name(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Fn(i) => Some(i.sig.ident.to_string()),
        syn::Item::Const(i) => Some(i.ident.to_string()),
        syn::Item::Static(i) => Some(i.ident.to_string()),
        syn::Item::Type(i) => Some(i.ident.to_string()),
        syn::Item::Struct(i) => Some(i.ident.to_string()),
        syn::Item::Enum(i) => Some(i.ident.to_string()),
        syn::Item::Union(i) => Some(i.ident.to_string()),
        syn::Item::Trait(i) => Some(i.ident.to_string()),
        _ => None,
    }
}

fn is_kernel_fn(item: &syn::Item) -> bool {
    match item {
        syn::Item::Fn(f) => f.attrs.iter().any(|attr| {
            attr.path()
                .segments
                .last()
                .is_some_and(|s| s.ident == "gpu_kernel")
        }),
        _ => false,
    }
}

pub(crate) fn is_own_root(name: &str, source: &CrateSource) -> bool {
    if matches!(name, "crate" | "self" | "super") {
        return true;
    }
    if RESERVED_ROOTS.contains(&name) {
        return false;
    }
    name == source.pkg_root || source.modules.get(name).copied() == Some(1)
}
