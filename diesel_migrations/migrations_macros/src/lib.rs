// Built-in Lints
#![deny(
    warnings,
    missing_debug_implementations,
    missing_copy_implementations
)]
// Clippy lints
#![cfg_attr(
    feature = "cargo-clippy",
    allow(
        option_map_unwrap_or_else,
        option_map_unwrap_or,
        match_same_arms,
        type_complexity
    )
)]
#![cfg_attr(
    feature = "cargo-clippy",
    warn(
        option_unwrap_used,
        print_stdout,
        wrong_pub_self_convention,
        mut_mut,
        non_ascii_literal,
        similar_names,
        unicode_not_nfc,
        enum_glob_use,
        if_not_else,
        items_after_statements,
        used_underscore_binding
    )
)]
#![cfg_attr(
    all(test, feature = "cargo-clippy"),
    allow(option_unwrap_used, result_unwrap_used)
)]
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
