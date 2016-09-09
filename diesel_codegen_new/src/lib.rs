#![feature(rustc_macro, rustc_macro_lib)]

#[macro_use]
extern crate quote;
extern crate rustc_macro;
extern crate syn;

mod ast_builder;
mod attr;
mod model;
mod util;

use rustc_macro::TokenStream;
use std::str::FromStr;
use syn::parse_item;

use self::attr::Attr;
use self::model::Model;

#[rustc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let input = input.replace("#[structural_match]", "");

    let item = parse_item(&input);
    let model = match Model::from_item(&item) {
        Ok(m) => m,
        Err(e) => panic!("#[derive(Queryable)] {}", e),
    };

    let struct_ty = &model.ty;
    let struct_name = &model.name;
    let ty_params = &model.generics.ty_params;
    let attrs = model.attrs;
    let lifetimes = &model.generics.lifetimes;

    let impl_tokens = quote!(Queryable! {
        (
            struct_name = #struct_name,
            struct_ty = #struct_ty,
            generics = (#(ty_params),*),
            lifetimes = (#(lifetimes),*),
        ),
        fields = [#(attrs)*],
    });
    TokenStream::from_str(&format!("{} {}", input, impl_tokens)).unwrap()
}
