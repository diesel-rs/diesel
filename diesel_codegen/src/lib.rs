#![feature(rustc_macro, rustc_macro_lib)]

#[macro_use]
extern crate quote;
extern crate rustc_macro;
extern crate syn;

use rustc_macro::TokenStream;

#[rustc_macro_derive(Dummy)]
pub fn placeholder(input: TokenStream) -> TokenStream {
    input
}
