// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../../clippy.toml")))]
#![cfg_attr(feature = "clippy",
            allow(option_map_unwrap_or_else, option_map_unwrap_or, match_same_arms,
                  type_complexity))]
#![cfg_attr(feature = "clippy",
            warn(option_unwrap_used, result_unwrap_used, print_stdout,
                 wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names,
                 unicode_not_nfc, enum_glob_use, if_not_else, items_after_statements,
                 used_underscore_binding))]
#![cfg_attr(all(test, feature = "clippy"), allow(option_unwrap_used, result_unwrap_used))]
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
//! For more information, consult the [`embed_migrations!`](../macro.embed_migrations.html) macro.
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
#[macro_use]
extern crate diesel;
extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate chrono;

#[cfg(feature = "barrel")]
extern crate barrel;

#[doc(hidden)]
pub mod connection;
#[doc(hidden)]
pub mod schema;

mod directory;
mod sql_plugin;
mod context;
mod compat;

// Legacy compatibility functions
pub use compat::*;

pub use directory::MigrationsDirectory;
pub use sql_plugin::{SqlFileMigration, SqlEmbeddedMigration};
pub use context::{MigrationContext, MarkedMigrations, PendingMigrations};

#[doc(inline)]
pub use self::connection::MigrationConnection;
pub use diesel::migration::*;

pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

