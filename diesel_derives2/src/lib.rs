#![recursion_limit = "1024"]
// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(option_map_unwrap_or_else, option_map_unwrap_or))]
#![cfg_attr(feature = "clippy",
            warn(wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, if_not_else, items_after_statements, used_underscore_binding))]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use proc_macro::TokenStream;

mod field;
mod meta;
mod model;
mod util;

mod as_changeset;

#[proc_macro_derive(AsChangeset,
                    attributes(table_name, primary_key, column_name, changeset_options))]
pub fn derive_as_changeset(input: TokenStream) -> TokenStream {
    expand_derive(input, as_changeset::derive)
}

fn expand_derive(input: TokenStream, f: fn(syn::DeriveInput) -> quote::Tokens) -> TokenStream {
    let item = syn::parse(input).unwrap();
    f(item).into()
}
