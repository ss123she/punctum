use std::collections::{BTreeSet, HashMap, HashSet};
use syn::visit::Visit;

use crate::scan::CrateSource;
use crate::uses::process_uses;

struct PathIdents<'a> {
    pkg_root: &'a str,
    idents: BTreeSet<String>,
}

impl<'a> PathIdents<'a> {
    fn new(pkg_root: &'a str) -> Self {
        Self {
            pkg_root,
            idents: BTreeSet::new(),
        }
    }
}

impl Visit<'_> for PathIdents<'_> {
    fn visit_attribute(&mut self, _attr: &syn::Attribute) {}

    fn visit_path(&mut self, path: &syn::Path) {
        if !path.segments.is_empty() {
            let first = path.segments[0].ident.to_string();
            if first == "crate" || first == "self" || first == "super" || first == self.pkg_root {
                if let Some(last) = path.segments.last() {
                    self.idents.insert(last.ident.to_string());
                }
            } else {
                self.idents.insert(first);
            }
        }
        syn::visit::visit_path(self, path);
    }
}

pub(crate) struct Resolved {
    pub(crate) items: Vec<usize>,
    pub(crate) impls: Vec<usize>,
    pub(crate) use_lines: BTreeSet<String>,
    pub(crate) extern_crates: BTreeSet<String>,
    pub(crate) warnings: Vec<String>,
}

fn build_name_index(source: &CrateSource) -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, item) in source.items.iter().enumerate() {
        index.entry(item.name.clone()).or_default().push(i);
    }
    index
}

fn impl_self_type_name(imp: &syn::ItemImpl) -> Option<String> {
    match &*imp.self_ty {
        syn::Type::Path(ty) => ty
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Type::Reference(reference) => match &*reference.elem {
            syn::Type::Path(ty) => ty
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn resolve_all(
    source: &CrateSource,
    kernel_indices: &[usize],
    direct_deps: &HashSet<String>,
) -> Resolved {
    let index = build_name_index(source);
    let mut items: BTreeSet<usize> = kernel_indices.iter().copied().collect();
    let mut impls: BTreeSet<usize> = BTreeSet::new();
    let mut warnings: BTreeSet<String> = BTreeSet::new();

    loop {
        let mut changed = false;

        let mut referenced: BTreeSet<String> = BTreeSet::new();
        for &i in &items {
            let mut visitor = PathIdents::new(&source.pkg_root);
            visitor.visit_item(&source.items[i].item);
            referenced.extend(visitor.idents);
        }
        for &i in &impls {
            let mut visitor = PathIdents::new(&source.pkg_root);
            visitor.visit_item_impl(&source.impls[i].item);
            referenced.extend(visitor.idents);
        }

        for name in &referenced {
            match index.get(name) {
                Some(matches) if matches.len() == 1 => changed |= items.insert(matches[0]),
                Some(matches) => {
                    warnings.insert(format!(
                        "shader dependency '{}' is ambiguous ({} definitions across scanned files); it will not be dragged",
                        name,
                        matches.len()
                    ));
                }
                None => {}
            }
        }

        for (i, imp) in source.impls.iter().enumerate() {
            if impls.contains(&i) {
                continue;
            }
            if let Some(type_name) = impl_self_type_name(&imp.item) {
                if let Some(matches) = index.get(&type_name) {
                    if matches.len() == 1 && items.contains(&matches[0]) {
                        changed |= impls.insert(i);
                    }
                }
            }
        }

        let uses = process_uses(
            source,
            &index,
            &items,
            &impls,
            &referenced,
            direct_deps,
            &mut warnings,
        );
        for name in &uses.extra_seeds {
            if let Some(matches) = index.get(name) {
                if matches.len() == 1 {
                    changed |= items.insert(matches[0]);
                }
            }
        }

        if !changed {
            return Resolved {
                items: items.into_iter().collect(),
                impls: impls.into_iter().collect(),
                use_lines: uses.lines,
                extern_crates: uses.extern_crates,
                warnings: warnings.into_iter().collect(),
            };
        }
    }
}
