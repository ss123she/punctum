use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn gpu_kernel(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
