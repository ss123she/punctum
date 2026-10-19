use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitInt, Token, parse_macro_input, punctuated::Punctuated};

#[proc_macro_attribute]
pub fn gpu_kernel(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let vis = &input_fn.vis;

    let (tx, ty, tz) = parse_threads(attr);

    let expanded = quote! {
        #[allow(non_upper_case_globals)]
        #vis const #fn_name: ::punctum::KernelDescriptor = ::punctum::KernelDescriptor {
            name: #fn_name_str,
            entry_point: #fn_name_str,
            workgroup_size: [#tx, #ty, #tz],
        };
    };

    TokenStream::from(expanded)
}

fn parse_threads(attr: TokenStream) -> (u32, u32, u32) {
    if attr.is_empty() {
        return (1, 1, 1);
    }

    let syn_args =
        syn::parse::Parser::parse(Punctuated::<LitInt, Token![,]>::parse_terminated, attr);

    if let Ok(args) = syn_args {
        let mut dims = [1u32; 3];
        for (i, arg) in args.iter().enumerate() {
            if let Ok(v) = arg.base10_parse() {
                dims[i] = v;
            }
        }
        (dims[0], dims[1], dims[2])
    } else {
        (1, 1, 1)
    }
}
