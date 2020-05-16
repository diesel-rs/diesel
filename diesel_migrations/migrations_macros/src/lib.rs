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
extern crate proc_macro;

mod embed_migrations;
mod migrations;

use proc_macro::TokenStream;

/// This macro will read your migrations at compile time, and embed a module you can use to execute
/// them at runtime without the migration files being present on the file system. This is useful if
/// you would like to use Diesel's migration infrastructure, but want to ship a single executable
/// file (such as for embedded applications). It can also be used to apply migrations to an in
/// memory database (Diesel does this for its own test suite).
///
/// You can optionally pass the path to the migrations directory to this macro. When left
/// unspecified, Diesel Codegen will search for the migrations directory in the same way that
/// Diesel CLI does. If specified, the path should be relative to the directory where `Cargo.toml`
/// resides.
///
/// # Examples
///
/// ```rust
/// # use diesel_migrations::embed_migrations;
/// # include!("../../../diesel/src/doctest_setup.rs");
/// # table! {
/// #   users {
/// #       id -> Integer,
/// #       name -> VarChar,
/// #   }
/// # }
/// #
/// # #[cfg(feature = "postgres")]
/// # embed_migrations!("../../migrations/postgresql");
/// # #[cfg(all(feature = "mysql", not(feature = "postgres")))]
/// # embed_migrations!("../../migrations/mysql");
/// # #[cfg(all(feature = "sqlite", not(any(feature = "postgres", feature = "mysql"))))]
/// embed_migrations!("../../migrations/sqlite");
///
/// fn main() {
///     let connection = establish_connection();
///
///     // This will run the necessary migrations.
///     embedded_migrations::run(&connection);
///
///     // By default the output is thrown out. If you want to redirect it to stdout, you
///     // should call embedded_migrations::run_with_output.
///     embedded_migrations::run_with_output(&connection, &mut std::io::stdout());
/// }
/// ```
#[proc_macro]
pub fn embed_migrations(input: TokenStream) -> TokenStream {
    embed_migrations::expand(input.to_string())
        .to_string()
        .parse()
        .unwrap()
}
