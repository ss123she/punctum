use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, LitInt, Pat, Token, Type, TypePath, TypeReference, TypeSlice, parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn gpu_kernel(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let vis = &input_fn.vis;

    let (tx, ty, tz) = match parse_threads(attr) {
        Ok(threads) => threads,
        Err(err) => return err.to_compile_error().into(),
    };

    let mut binding_tokens = Vec::new();
    let mut current_binding = 0u32;

    for input in &input_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if is_spirv_builtin(pat_type) {
                continue;
            }

            let arg_name = match &*pat_type.pat {
                Pat::Ident(id) => id.ident.to_string(),
                _ => format!("arg_{current_binding}"),
            };

            let binding_index = extract_binding_index(pat_type).unwrap_or(current_binding);
            let res_type_tokens = classify_type(&pat_type.ty);

            binding_tokens.push(quote! {
                ::punctum::BindingDescriptor {
                    binding: #binding_index,
                    name: #arg_name,
                    resource_type: #res_type_tokens,
                }
            });

            current_binding = binding_index + 1;
        }
    }

    let mut typecheck_fn = input_fn.clone();
    for input in &mut typecheck_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            pat_type.attrs.retain(|a| !a.path().is_ident("spirv"));
        }
    }
    typecheck_fn.attrs.retain(|a| !a.path().is_ident("spirv"));

    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        #vis const #fn_name: ::punctum::KernelDescriptor = ::punctum::KernelDescriptor {
            name: #fn_name_str,
            entry_point: #fn_name_str,
            workgroup_size: [#tx, #ty, #tz],
            bindings: &[
                #(#binding_tokens),*
            ],
        };

        #[doc(hidden)]
        #[allow(dead_code, unused_variables, unused_mut, non_snake_case)]
        const _: () = {
            #typecheck_fn
        };
    };

    TokenStream::from(expanded)
}

fn parse_threads(attr: TokenStream) -> syn::Result<(u32, u32, u32)> {
    if attr.is_empty() {
        return Ok((1, 1, 1));
    }

    let args = syn::parse::Parser::parse(Punctuated::<LitInt, Token![,]>::parse_terminated, attr)?;

    let mut dims = [1u32; 3];
    for (i, arg) in args.iter().enumerate() {
        if i >= 3 {
            return Err(syn::Error::new(
                arg.span(),
                "expected at most 3 workgroup dimensions",
            ));
        }
        dims[i] = arg.base10_parse()?;
    }

    Ok((dims[0], dims[1], dims[2]))
}

fn is_spirv_builtin(pat: &syn::PatType) -> bool {
    pat.attrs.iter().any(|attr| {
        if !attr.path().is_ident("spirv") {
            return false;
        }
        let tokens = attr.meta.to_token_stream().to_string();
        tokens.contains("global_invocation_id")
            || tokens.contains("local_invocation_id")
            || tokens.contains("workgroup_id")
            || tokens.contains("num_workgroups")
            || tokens.contains("subgroup")
    })
}

fn extract_binding_index(pat: &syn::PatType) -> Option<u32> {
    for attr in &pat.attrs {
        if attr.path().is_ident("spirv") {
            let s = attr.meta.to_token_stream().to_string();
            if let Some(pos) = s.find("binding") {
                let rest = &s[pos + "binding".len()..];
                let digits: String = rest
                    .chars()
                    .skip_while(|c| *c == ' ' || *c == '=')
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(val) = digits.parse::<u32>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

fn classify_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Reference(TypeReference {
            mutability, elem, ..
        }) => {
            let is_mut = mutability.is_some();

            if let Type::Path(TypePath { path, .. }) = &**elem {
                let ident = &path.segments.last().unwrap().ident.to_string();
                if ident.contains("StorageImage") {
                    return quote! {
                        ::punctum::KernelResourceType::StorageTexture {
                            write: true,
                        }
                    };
                }

                if ident.contains("SampledImage") || ident.contains("Image") {
                    return quote! {
                        ::punctum::KernelResourceType::SampledTexture
                    };
                }

                if ident.contains("Sampler") {
                    quote! {
                        ::punctum::KernelResourceType::Sampler
                    };
                }
            } else if let Type::Slice(TypeSlice { .. }) = &**elem {
                let read_only = !is_mut;
                return quote! {
                    ::punctum::KernelResourceType::StorageBuffer {
                        read_only: #read_only,
                    }
                };
            }

            quote! {
                ::punctum::KernelResourceType::UniformBuffer
            }
        }
        _ => {
            quote! {
                ::punctum::KernelResourceType::UniformBuffer
            }
        }
    }
}
