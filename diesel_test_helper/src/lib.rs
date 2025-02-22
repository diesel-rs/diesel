extern crate proc_macro2;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Since sqlite wasm support has been added, #[wasm_bindgen_test] needs
/// to be used in the wasm environment. This macro is designed to solve platform test differences.
#[proc_macro_attribute]
pub fn test(_: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        sig,
        vis,
        block,
        attrs,
    } = parse_macro_input!(item as ItemFn);

    let cfgs = quote! {
        #[cfg_attr(all(target_family = "wasm", target_os = "unknown"), wasm_bindgen_test::wasm_bindgen_test)]
        #[cfg_attr(not(all(target_family = "wasm", target_os = "unknown")), test)]
    };
    quote!(
        #cfgs
        #(#attrs)*
        #vis #sig {
            #block
        }
    )
    .into()
}
