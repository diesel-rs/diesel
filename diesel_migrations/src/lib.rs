// Built-in Lints
// Clippy lints
#![allow(clippy::needless_pass_by_value, clippy::map_unwrap_or)]
#![warn(
    missing_docs,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding,
    missing_copy_implementations
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
//! Individual migrations should be a folder containing two files, `up.sql` and `down.sql`.
//! `up.sql` will be used to run the migration, while `down.sql` will be used for reverting it. The
//! folder itself should have the structure `{version}_{migration_name}`. It is recommended that
//! you use the timestamp of creation for the version.
//!
//! Migrations can either be run with the CLI or embedded into the compiled application
//! and executed with code, for example right after establishing a database connection.
//! For more information, consult the [`embed_migrations!`] macro.
//!

mod embedded_migrations;
mod errors;
mod file_based_migrations;
mod migration_harness;

pub use crate::embedded_migrations::EmbeddedMigrations;
pub use crate::file_based_migrations::FileBasedMigrations;
pub use crate::migration_harness::{HarnessWithOutput, MigrationHarness};
pub use migrations_macros::embed_migrations;

#[doc(hidden)]
pub use crate::embedded_migrations::{EmbeddedMigration, EmbeddedName};
#[doc(hidden)]
pub use crate::errors::MigrationError;
#[doc(hidden)]
pub use crate::file_based_migrations::TomlMetadataWrapper;
