#![feature(rustc_macro, rustc_macro_lib)]

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
mod identifiable;
mod model;
mod queryable;
mod util;

use rustc_macro::TokenStream;
use std::str::FromStr;
use syn::parse_item;

#[rustc_macro_derive(Queryable)]
pub fn derive_queryable(input: TokenStream) -> TokenStream {
    expand_derive(input, queryable::derive_queryable)
}

#[rustc_macro_derive(Identifiable)]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    expand_derive(input, identifiable::derive_identifiable)
}

fn expand_derive(input: TokenStream, f: fn(syn::Item) -> quote::Tokens) -> TokenStream {
    let input = input.to_string();
    // FIXME: https://github.com/rust-lang/rust/issues/35900#issuecomment-245971366
    let input = input.replace("#[structural_match]", "");

    let item = parse_item(&input);
    let output = f(item);
    TokenStream::from_str(&format!("{} {}", input, output)).unwrap()
}
