// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![allow(
    clippy::needless_pass_by_value,
    clippy::option_map_unwrap_or_else,
    clippy::option_map_unwrap_or
)]
#![warn(
    clippy::wrong_pub_self_convention,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding
)]
//! Provides functions for maintaining database schema.
//!
//! A database migration always provides procedures to update the schema, as well as to revert
//! itself. Diesel's migrations are versioned, and run in order. Diesel also takes care of tracking
//! which migrations have already been run automatically. Your migrations don't need to be
//! idempotent, as Diesel will ensure no migration is run twice unless it has been reverted.
//!
//! Migrations should be placed in a `/migrations` directory at the root of your project (the same
//! directory as `Cargo.toml`). When any of these functions are run, Diesel will search for the
//! migrations directory in the current directory and its parents, stopping when it finds the
//! directory containing `Cargo.toml`.
//!
//! Individual migrations should be a folder containing exactly two files, `up.sql` and `down.sql`.
//! `up.sql` will be used to run the migration, while `down.sql` will be used for reverting it. The
//! folder itself should have the structure `{version}_{migration_name}`. It is recommended that
//! you use the timestamp of creation for the version.
//!
//! Migrations can either be run with the CLI or embedded into the compiled application
//! and executed with code, for example right after establishing a database connection.
//! For more information, consult the [`embed_migrations!`](macro.embed_migrations.html) macro.
//!
//! ## Example
//!
//! ```text
//! # Directory Structure
//! - 20151219180527_create_users
//!     - up.sql
//!     - down.sql
//! - 20160107082941_create_posts
//!     - up.sql
//!     - down.sql
//! ```
//!
//! ```sql
//! -- 20151219180527_create_users/up.sql
//! CREATE TABLE users (
//!   id SERIAL PRIMARY KEY,
//!   name VARCHAR NOT NULL,
//!   hair_color VARCHAR
//! );
//! ```
//!
//! ```sql
//! -- 20151219180527_create_users/down.sql
//! DROP TABLE users;
//! ```
//!
//! ```sql
//! -- 20160107082941_create_posts/up.sql
//! CREATE TABLE posts (
//!   id SERIAL PRIMARY KEY,
//!   user_id INTEGER NOT NULL,
//!   title VARCHAR NOT NULL,
//!   body TEXT
//! );
//! ```
//!
//! ```sql
//! -- 20160107082941_create_posts/down.sql
//! DROP TABLE posts;
//! ```

extern crate migrations_internals;
extern crate migrations_macros;
#[doc(inline)]
pub use migrations_internals::any_pending_migrations;
#[doc(inline)]
pub use migrations_internals::find_migrations_directory;
#[doc(inline)]
pub use migrations_internals::mark_migrations_in_directory;
#[doc(inline)]
pub use migrations_internals::migration_from;
#[doc(inline)]
pub use migrations_internals::migration_paths_in_directory;
#[doc(inline)]
pub use migrations_internals::name;
#[doc(inline)]
pub use migrations_internals::revert_latest_migration;
#[doc(inline)]
pub use migrations_internals::revert_latest_migration_in_directory;
#[doc(inline)]
pub use migrations_internals::revert_migration_with_version;
#[doc(inline)]
pub use migrations_internals::run_migration_with_version;
#[doc(inline)]
pub use migrations_internals::run_migrations;
#[doc(inline)]
pub use migrations_internals::run_pending_migrations;
#[doc(inline)]
pub use migrations_internals::run_pending_migrations_in_directory;
#[doc(inline)]
pub use migrations_internals::search_for_migrations_directory;
#[doc(inline)]
pub use migrations_internals::setup_database;
#[doc(inline)]
pub use migrations_internals::version_from_path;
#[doc(inline)]
pub use migrations_internals::Migration;
#[doc(inline)]
pub use migrations_internals::MigrationConnection;
#[doc(inline)]
pub use migrations_internals::MigrationError;
#[doc(inline)]
pub use migrations_internals::MigrationName;
#[doc(inline)]
pub use migrations_internals::RunMigrationsError;
#[doc(hidden)]
pub use migrations_macros::*;

pub mod connection {
    #[doc(inline)]
    pub use migrations_internals::connection::MigrationConnection;
}

#[macro_export]
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
/// # #[macro_use] extern crate diesel;
/// # #[macro_use] extern crate diesel_migrations;
/// # include!("../../diesel/src/doctest_setup.rs");
/// # table! {
/// #   users {
/// #       id -> Integer,
/// #       name -> VarChar,
/// #   }
/// # }
/// #
/// # #[cfg(feature = "postgres")]
/// # embed_migrations!("../migrations/postgresql");
/// # #[cfg(all(feature = "mysql", not(feature = "postgres")))]
/// # embed_migrations!("../migrations/mysql");
/// # #[cfg(all(feature = "sqlite", not(any(feature = "postgres", feature = "mysql"))))]
/// embed_migrations!("../migrations/sqlite");
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
macro_rules! embed_migrations {
    () => {
        #[allow(dead_code)]
        mod embedded_migrations {
            #[derive(EmbedMigrations)]
            struct _Dummy;
        }
    };

    ($migrations_path:expr) => {
        #[allow(dead_code)]
        mod embedded_migrations {
            #[derive(EmbedMigrations)]
            #[embed_migrations_options(migrations_path=$migrations_path)]
            struct _Dummy;
        }
    };
}
