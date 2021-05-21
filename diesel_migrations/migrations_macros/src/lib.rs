// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::type_complexity,
    clippy::needless_doctest_main
)]
#![warn(
    clippy::unwrap_used,
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

/// This macro will read your migrations at compile time, and create a constant value containing
/// an embedded list of all your migrations as available at compile time.
/// This is useful if you would like to use Diesel's migration infrastructure, but want to ship a single executable
/// file (such as for embedded applications). It can also be used to apply migrations to an in
/// memory database (Diesel does this for its own test suite).
///
/// You can optionally pass the path to the migrations directory to this macro. When left
/// unspecified, Diesel will search for the migrations directory in the same way that
/// Diesel CLI does. If specified, the path should be relative to the directory where `Cargo.toml`
/// resides.
///
/// # Examples
///
/// ```rust
/// use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
/// # use std::error::Error;
/// # include!("../../../diesel/src/doctest_setup.rs");
/// #
/// # #[cfg(feature = "postgres")]
/// # fn migration_connection() -> diesel::PgConnection {
/// #    let connection_url = database_url_from_env("PG_DATABASE_URL");
/// #    let mut conn = diesel::PgConnection::establish(&connection_url).unwrap();
/// #    conn.begin_test_transaction().unwrap();
/// #    conn
/// # }
/// #
/// # #[cfg(feature = "sqlite")]
/// # fn migration_connection() -> diesel::SqliteConnection {
/// #    let connection_url = database_url_from_env("SQLITE_DATABASE_URL");
/// #    let mut conn = diesel::SqliteConnection::establish(&connection_url).unwrap();
/// #    conn.begin_test_transaction().unwrap();
/// #    conn
/// # }
/// #
/// # #[cfg(feature = "mysql")]
/// # fn migration_connection() -> diesel::MysqlConnection {
/// #    let connection_url = database_url_from_env("MYSQL_DATABASE_URL");
/// #    let mut conn = diesel::MysqlConnection::establish(&connection_url).unwrap();
/// #    conn
/// # }
/// #
/// #
/// # #[cfg(feature = "postgres")]
/// pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/postgresql");
/// # #[cfg(all(feature = "mysql", not(feature = "postgres")))]
/// # pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/mysql");
/// # #[cfg(all(feature = "sqlite", not(any(feature = "postgres", feature = "mysql"))))]
/// # pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations/sqlite");
///
/// # fn main() {
/// #     let connection = &mut migration_connection();
/// #     run_migrations(connection).unwrap();
/// # }
///
/// fn run_migrations(connection: &mut impl MigrationHarness<DB>) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
/// #   #[cfg(feature = "mysql")]
/// #   connection.revert_all_migrations(MIGRATIONS)?;
///
///     // This will run the necessary migrations.
///     //
///     // See the documentation for `MigrationHarness` for
///     // all available methods.
///     connection.run_pending_migrations(MIGRATIONS)?;
///
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn embed_migrations(input: TokenStream) -> TokenStream {
    embed_migrations::expand(input.to_string())
        .to_string()
        .parse()
        .expect("Failed create embedded migrations instance")
}
