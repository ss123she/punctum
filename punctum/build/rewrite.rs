use proc_macro2::Span;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit_mut::VisitMut;
use syn::{LitInt, Meta, Token};

use crate::scan::{CrateSource, is_own_root};

struct PathRewriter<'a> {
    source: &'a CrateSource,
}

impl PathRewriter<'_> {
    fn collapse_use_tree(&mut self, tree: &mut syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                let first = path.ident.to_string();
                if is_own_root(&first, self.source) {
                    let leaf = extract_use_leaf(tree);
                    *tree = syn::parse_quote!(self :: #leaf);
                    if let syn::UseTree::Path(path) = tree {
                        self.collapse_use_tree(&mut path.tree);
                    }
                } else {
                    self.collapse_use_tree(&mut path.tree);
                }
            }
            syn::UseTree::Group(group) => {
                for item in &mut group.items {
                    self.collapse_use_tree(item);
                }
            }
            _ => {}
        }
    }
}

fn extract_use_leaf(tree: &mut syn::UseTree) -> syn::UseTree {
    if let syn::UseTree::Path(path) = tree {
        extract_use_leaf(&mut path.tree)
    } else {
        std::mem::replace(
            tree,
            syn::UseTree::Name(syn::UseName {
                ident: syn::Ident::new("__punctum_removed", Span::call_site()),
            }),
        )
    }
}

impl VisitMut for PathRewriter<'_> {
    fn visit_item_use_mut(&mut self, use_item: &mut syn::ItemUse) {
        self.collapse_use_tree(&mut use_item.tree);
    }

    fn visit_path_mut(&mut self, path: &mut syn::Path) {
        if path.segments.len() >= 2 {
            let first = path.segments[0].ident.to_string();
            if is_own_root(&first, self.source) {
                let last = path.segments.last().unwrap().clone();
                let mut segments = Punctuated::new();
                segments.push_value(last);
                path.segments = segments;
                path.leading_colon = None;
                syn::visit_mut::visit_path_mut(self, path);
                return;
            }
        }

        syn::visit_mut::visit_path_mut(self, path);
    }
}

fn strip_gpu_kernel_attr(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| {
        attr.path()
            .segments
            .last()
            .is_none_or(|segment| segment.ident != "gpu_kernel")
    });
}

pub(crate) fn strip_gpu_kernel_from_item(item: &mut syn::Item) {
    match item {
        syn::Item::Fn(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Const(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Static(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Type(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Struct(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Enum(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Union(i) => strip_gpu_kernel_attr(&mut i.attrs),
        syn::Item::Trait(i) => strip_gpu_kernel_attr(&mut i.attrs),
        _ => {}
    }
}

pub(crate) fn transform_kernel(func: &syn::ItemFn) -> syn::ItemFn {
    let mut f = func.clone();
    let (tx, ty, tz) = parse_kernel_threads(&f.attrs);
    strip_gpu_kernel_attr(&mut f.attrs);
    f.attrs
        .push(syn::parse_quote!(#[spirv(compute(threads(#tx, #ty, #tz)))]));
    f
}

pub(crate) fn emit_item(item: &mut syn::Item, source: &CrateSource) -> String {
    let mut rewriter = PathRewriter { source };
    rewriter.visit_item_mut(item);
    item.to_token_stream().to_string()
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
