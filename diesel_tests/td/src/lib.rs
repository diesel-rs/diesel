extern crate proc_macro2;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, token::Async, ItemFn};

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
        #[cfg_attr(all(target_family = "wasm", target_os = "unknown", feature = "sqlite"), td::sqlite_wasm)]
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

/// Sqlite wasm requires asynchronous initialization, so this macro
/// turns the function into asynchronous and provides an initialization method
#[proc_macro_attribute]
pub fn sqlite_wasm(_: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn {
        mut sig,
        vis,
        block,
        attrs,
    } = parse_macro_input!(item as ItemFn);

    let prepare = quote! {
        crate::wasm_export::init_sqlite().await.unwrap();
    };

    sig.asyncness = Some(Async::default());

    quote!(
        #(#attrs)*
        #vis #sig {
            #prepare
            #block
        }
    )
    .into()
}
