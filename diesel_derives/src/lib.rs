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
#[macro_use]
extern crate quote;
extern crate syn;

mod ast_builder;
mod sql_type;
mod util;

use proc_macro::TokenStream;
use syn::parse_derive_input;

#[proc_macro_derive(SqlType, attributes(postgres, sqlite_type, mysql_type))]
pub fn derive_sql_type(input: TokenStream) -> TokenStream {
    expand_derive(input, sql_type::derive)
}

fn expand_derive(input: TokenStream, f: fn(syn::DeriveInput) -> quote::Tokens) -> TokenStream {
    let item = parse_derive_input(&input.to_string()).unwrap();
    f(item).to_string().parse().unwrap()
}
