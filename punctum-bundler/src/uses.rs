use quote::ToTokens;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::scan::{CrateSource, RESERVED_ROOTS, is_own_root};

pub(crate) struct UsePass {
    pub(crate) lines: BTreeSet<String>,
    pub(crate) extern_crates: BTreeSet<String>,
    pub(crate) extra_seeds: BTreeSet<String>,
}

struct UseLeaf {
    name: String,
    rename: Option<String>,
    is_glob: bool,
}

fn use_tree_root(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => Some(path.ident.to_string()),
        _ => None,
    }
}

fn use_leaves(tree: &syn::UseTree, prefix: Option<&str>) -> Vec<UseLeaf> {
    match tree {
        syn::UseTree::Path(path) => {
            let ident = path.ident.to_string();
            if ident == "self" {
                vec![UseLeaf {
                    name: prefix.unwrap_or("self").to_string(),
                    rename: None,
                    is_glob: false,
                }]
            } else {
                use_leaves(&path.tree, Some(&ident))
            }
        }
        syn::UseTree::Name(name) => {
            if name.ident == "self" {
                vec![UseLeaf {
                    name: prefix.unwrap_or("self").to_string(),
                    rename: None,
                    is_glob: false,
                }]
            } else {
                vec![UseLeaf {
                    name: name.ident.to_string(),
                    rename: None,
                    is_glob: false,
                }]
            }
        }
        syn::UseTree::Rename(rename) => vec![UseLeaf {
            name: if rename.ident == "self" {
                prefix.unwrap_or("self").to_string()
            } else {
                rename.ident.to_string()
            },
            rename: Some(rename.rename.to_string()),
            is_glob: false,
        }],
        syn::UseTree::Glob(_) => vec![UseLeaf {
            name: prefix.unwrap_or("*").to_string(),
            rename: None,
            is_glob: true,
        }],
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|tree| use_leaves(tree, prefix))
            .collect(),
    }
}

pub(crate) fn process_uses(
    source: &CrateSource,
    index: &HashMap<String, Vec<usize>>,
    items: &BTreeSet<usize>,
    impls: &BTreeSet<usize>,
    referenced: &BTreeSet<String>,
    direct_deps: &HashSet<String>,
    skip_roots: &[String],
    warnings: &mut BTreeSet<String>,
) -> UsePass {
    let mut lines: BTreeSet<String> = BTreeSet::new();
    let mut extern_crates: BTreeSet<String> = BTreeSet::new();
    let mut extra_seeds: BTreeSet<String> = BTreeSet::new();

    lines.insert("use spirv_std::spirv;".to_string());
    let mut bound_names: HashMap<String, String> =
        HashMap::from([("spirv".to_string(), "spirv_std::spirv".to_string())]);

    let mut contributing: HashSet<&str> = HashSet::new();
    for &i in items {
        contributing.insert(source.items[i].file.as_str());
    }
    for &i in impls {
        contributing.insert(source.impls[i].file.as_str());
    }

    for (file, use_item) in &source.uses {
        if !contributing.contains(file.as_str()) {
            continue;
        }

        let root = use_tree_root(&use_item.tree).unwrap_or_default();

        if is_own_root(&root, source) {
            for leaf in use_leaves(&use_item.tree, None) {
                if let Some(alias) = &leaf.rename {
                    if !referenced.contains(alias) {
                        continue;
                    }
                    match index.get(leaf.name.as_str()) {
                        Some(matches) if matches.len() == 1 => {
                            extra_seeds.insert(leaf.name.clone());
                            lines.insert(format!("use self::{} as {};", leaf.name, alias));
                            track_bound(
                                &mut bound_names,
                                alias,
                                &format!("{}::{}", root, leaf.name),
                                file,
                                warnings,
                            );
                        }
                        Some(matches) => {
                            warnings.insert(format!(
                                "shader dependency '{}::{} as {}' ({}) is ambiguous ({} definitions); it will not be dragged",
                                root,
                                leaf.name,
                                alias,
                                file,
                                matches.len()
                            ));
                        }
                        None => {
                            warnings.insert(format!(
                                "shader dependency '{}::{} as {}' ({}) was not found in scanned sources",
                                root, leaf.name, alias, file
                            ));
                        }
                    }
                }
            }
        } else if source.modules.get(&root).copied().unwrap_or(0) > 1 {
            warnings.insert(format!(
                "shader import in {file} goes through module '{root}' declared in multiple files; it cannot be flattened"
            ));
        } else if root == "std" || skip_roots.iter().any(|s| s == &root) {
        } else {
            let leaves = use_leaves(&use_item.tree, None);
            let root_referenced = !root.is_empty() && referenced.contains(&root);
            let keep = root_referenced
                || leaves.iter().any(|leaf| leaf.is_glob)
                || leaves.iter().any(|leaf| {
                    referenced.contains(&leaf.name)
                        || leaf
                            .rename
                            .as_ref()
                            .is_some_and(|alias| referenced.contains(alias))
                });

            if keep {
                let mut expanded = Vec::new();
                expand_use_tree(&use_item.tree, "", &mut expanded);
                for path in expanded {
                    lines.insert(path);
                }

                if !root.is_empty() {
                    if root != "spirv_std" && !skip_roots.iter().any(|s| s == &root) {
                        extern_crates.insert(root);
                    }
                } else {
                    for leaf in &leaves {
                        if !leaf.is_glob
                            && leaf.rename.is_none()
                            && leaf.name != "self"
                            && leaf.name != "spirv_std"
                            && !RESERVED_ROOTS.contains(&leaf.name.as_str())
                        {
                            extern_crates.insert(leaf.name.clone());
                        }
                    }
                }
            }
        }
    }

    for name in referenced {
        if index.contains_key(name)
            || source.modules.contains_key(name)
            || *name == source.pkg_root
            || name == "spirv_std"
            || RESERVED_ROOTS.contains(&name.as_str())
        {
            continue;
        }
        if skip_roots.iter().any(|s| s == name) {
            warnings.insert(format!(
                "shader code references host crate '{name}'; it cannot be dragged into the shadow crate"
            ));
            continue;
        }
        if direct_deps.contains(name) {
            extern_crates.insert(name.clone());
        }
    }

    UsePass {
        lines,
        extern_crates,
        extra_seeds,
    }
}

fn expand_use_tree(tree: &syn::UseTree, prefix: &str, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(p) => {
            let next_prefix = if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{}::{}", prefix, p.ident)
            };
            expand_use_tree(&p.tree, &next_prefix, out);
        }
        syn::UseTree::Name(n) => {
            if n.ident == "self" {
                out.push(format!("use {};", prefix));
            } else if prefix.is_empty() {
                out.push(format!("use {};", n.ident));
            } else {
                out.push(format!("use {}::{};", prefix, n.ident));
            }
        }
        syn::UseTree::Rename(r) => {
            if r.ident == "self" {
                out.push(format!("use {} as {};", prefix, r.rename));
            } else if prefix.is_empty() {
                out.push(format!("use {} as {};", r.ident, r.rename));
            } else {
                out.push(format!("use {}::{} as {};", prefix, r.ident, r.rename));
            }
        }
        syn::UseTree::Glob(_) => {
            if prefix.is_empty() {
                out.push("use *;".to_string());
            } else {
                out.push(format!("use {}::*; ", prefix));
            }
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                expand_use_tree(item, prefix, out);
            }
        }
    }
}

fn track_bound(
    bound: &mut HashMap<String, String>,
    name: &str,
    source_path: &str,
    file: &str,
    warnings: &mut BTreeSet<String>,
) {
    match bound.get(name) {
        Some(existing) if existing != source_path => {
            warnings.insert(format!(
                "shader import conflict: '{name}' is imported as both {existing} and {source_path} (from {file})"
            ));
        }
        _ => {
            bound.insert(name.to_string(), source_path.to_string());
        }
    }
}
