#![feature(rustc_macro, rustc_macro_lib)]
#![deny(warnings)]

macro_rules! t {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}", e),
        }
    };
}

#[macro_use]
extern crate quote;
extern crate rustc_macro;
extern crate syn;

mod ast_builder;
mod attr;
mod model;
mod queryable;
mod util;

use rustc_macro::TokenStream;
use syn::parse_macro_input;

#[rustc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

fn expand_derive(input: TokenStream, f: fn(syn::MacroInput) -> quote::Tokens) -> TokenStream {
    let item = parse_macro_input(&input.to_string()).unwrap();
    let output = f(item.clone());

    quote!(#item #output).to_string().parse().unwrap()
}
