// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::option_map_unwrap_or_else,
    clippy::option_map_unwrap_or,
    clippy::match_same_arms,
    clippy::type_complexity
)]
#![warn(
    clippy::option_unwrap_used,
    clippy::print_stdout,
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
#![cfg_attr(test, allow(clippy::option_unwrap_used, clippy::result_unwrap_used))]
extern crate migrations_internals;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

mod embed_migrations;
mod migrations;
mod util;

use proc_macro::TokenStream;
use syn::parse_derive_input;

#[proc_macro_derive(EmbedMigrations, attributes(embed_migrations_options))]
pub fn derive_embed_migrations(input: TokenStream) -> TokenStream {
    let item = parse_derive_input(&input.to_string()).unwrap();
    embed_migrations::derive_embed_migrations(&item)
        .to_string()
        .parse()
        .unwrap()
}
