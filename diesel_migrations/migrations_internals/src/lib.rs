// Built-in Lints
#![deny(warnings, missing_debug_implementations, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
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

mod migration;
#[doc(hidden)]
pub mod connection;
mod migration_error;
#[doc(hidden)]
pub mod schema;

#[doc(inline)]
pub use self::connection::MigrationConnection;
#[doc(inline)]
pub use self::migration::*;
pub use self::migration_error::*;

use std::fs::DirEntry;
use std::io::{stdout, Write};

use diesel::expression_methods::*;
use diesel::{ExecuteDsl, FilterDsl};
use self::schema::__diesel_schema_migrations::dsl::*;
use diesel::{Connection, QueryResult};

use std::env;
use std::path::{Path, PathBuf};


pub static TIMESTAMP_FORMAT: &'static str = "%Y-%m-%d-%H%M%S";

/// Runs all migrations that have not yet been run. This function will print all progress to
/// stdout. This function will return an `Err` if some error occurs reading the migrations, or if
/// any migration fails to run. Each migration is run in its own transaction, so some migrations
/// may be committed, even if a later migration fails to run.
///
/// It should be noted that this runs all migrations that have not already been run, regardless of
/// whether or not their version is later than the latest run migration. This is generally not a
/// problem, and eases the more common case of two developers generating independent migrations on
/// a branch. Whoever created the second one will eventually need to run the first when both
/// branches are merged.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
pub fn run_pending_migrations<Conn>(conn: &Conn) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
{
    let migrations_dir = try!(find_migrations_directory());
    run_pending_migrations_in_directory(conn, &migrations_dir, &mut stdout())
}

#[doc(hidden)]
pub fn run_pending_migrations_in_directory<Conn>(
    conn: &Conn,
    migrations_dir: &Path,
    output: &mut Write,
) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
{
    let all_migrations = try!(migrations_in_directory(migrations_dir));
    run_migrations(conn, all_migrations, output)
}

/// Compares migrations found in `migrations_dir` to those that have been applied.
/// Returns a list of pathbufs and whether they have been applied.
pub fn mark_migrations_in_directory<Conn>(
    conn: &Conn,
    migrations_dir: &Path,
) -> Result<Vec<(Option<PathBuf>, bool)>, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    let migrations = migrations_in_directory(migrations_dir)?;
    setup_database(conn)?;
    let already_run = conn.previously_run_migration_versions()?;
    let migrations = migrations
        .into_iter()
        .map(|m| {
            let applied = already_run.contains(&m.version().to_string());
            (m.file_path().map(|p| p.to_path_buf()), applied)
        })
        .collect();
    Ok(migrations)
}

// Returns true if there are outstanding migrations in the migrations directory, otherwise
// returns false. Returns an `Err` if there are problems with migration setup.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
pub fn any_pending_migrations<Conn>(conn: &Conn) -> Result<bool, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    let migrations_dir = find_migrations_directory()?;
    let all_migrations = migrations_in_directory(&migrations_dir)?;
    let already_run = conn.previously_run_migration_versions()?;

    let pending = all_migrations
        .into_iter()
        .any(|m| !already_run.contains(&m.version().to_string()));

    Ok(pending)
}

/// Reverts the last migration that was run. Returns the version that was reverted. Returns an
/// `Err` if no migrations have ever been run.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
pub fn revert_latest_migration<Conn>(conn: &Conn) -> Result<String, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    let migrations_dir = try!(find_migrations_directory());
    revert_latest_migration_in_directory(conn, &migrations_dir)
}

pub fn revert_latest_migration_in_directory<Conn>(
    conn: &Conn,
    path: &Path,
) -> Result<String, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    try!(setup_database(conn));
    let latest_migration_version = conn.latest_run_migration_version()?.ok_or_else(|| {
        RunMigrationsError::MigrationError(MigrationError::NoMigrationRun)
    })?;
    revert_migration_with_version(conn, path, &latest_migration_version, &mut stdout())
        .map(|_| latest_migration_version)
}

#[doc(hidden)]
pub fn revert_migration_with_version<Conn: Connection>(
    conn: &Conn,
    migrations_dir: &Path,
    ver: &str,
    output: &mut Write,
) -> Result<(), RunMigrationsError> {
    migration_with_version(migrations_dir, ver)
        .map_err(|e| e.into())
        .and_then(|m| revert_migration(conn, m, output))
}

#[doc(hidden)]
pub fn run_migration_with_version<Conn>(
    conn: &Conn,
    migrations_dir: &Path,
    ver: &str,
    output: &mut Write,
) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
{
    migration_with_version(migrations_dir, ver)
        .map_err(|e| e.into())
        .and_then(|m| run_migration(conn, &*m, output))
}

fn migration_with_version(
    migrations_dir: &Path,
    ver: &str,
) -> Result<Box<Migration>, MigrationError> {
    let all_migrations = try!(migrations_in_directory(migrations_dir));
    let migration = all_migrations.into_iter().find(|m| m.version() == ver);
    match migration {
        Some(m) => Ok(m),
        None => Err(MigrationError::UnknownMigrationVersion(ver.into())),
    }
}

#[doc(hidden)]
pub fn setup_database<Conn: Connection>(conn: &Conn) -> QueryResult<usize> {
    create_schema_migrations_table_if_needed(conn)
}

fn create_schema_migrations_table_if_needed<Conn: Connection>(conn: &Conn) -> QueryResult<usize> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (\
         version VARCHAR(50) PRIMARY KEY NOT NULL,\
         run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP\
         )",
    )
}

#[doc(hidden)]
pub fn migration_paths_in_directory(path: &Path) -> Result<Vec<DirEntry>, MigrationError> {
    try!(path.read_dir())
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => return Some(Err(e.into())),
            };
            if entry.file_name().to_string_lossy().starts_with('.') {
                None
            } else {
                Some(Ok(entry))
            }
        })
        .collect()
}

fn migrations_in_directory(path: &Path) -> Result<Vec<Box<Migration>>, MigrationError> {
    use self::migration::migration_from;

    try!(migration_paths_in_directory(path))
        .iter()
        .map(|e| migration_from(e.path()))
        .collect()
}

/// Run all pending migrations in the given list. Apps should likely be calling
/// `run_pending_migrations` or `run_pending_migrations_in_directory` instead.
pub fn run_migrations<Conn, List>(
    conn: &Conn,
    migrations: List,
    output: &mut Write,
) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
    List: IntoIterator,
    List::Item: Migration,
{
    try!(setup_database(conn));
    let already_run = try!(conn.previously_run_migration_versions());
    let mut pending_migrations: Vec<_> = migrations
        .into_iter()
        .filter(|m| !already_run.contains(&m.version().to_string()))
        .collect();

    pending_migrations.sort_by(|a, b| a.version().cmp(b.version()));
    for migration in pending_migrations {
        try!(run_migration(conn, &migration, output));
    }
    Ok(())
}

fn run_migration<Conn>(
    conn: &Conn,
    migration: &Migration,
    output: &mut Write,
) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
{
    conn.transaction(|| {
        if migration.version() != "00000000000000" {
            try!(writeln!(
                output,
                "Running migration {}",
                migration.version()
            ));
        }
        try!(migration.run(conn));
        try!(conn.insert_new_migration(migration.version()));
        Ok(())
    })
}

fn revert_migration<Conn: Connection>(
    conn: &Conn,
    migration: Box<Migration>,
    output: &mut Write,
) -> Result<(), RunMigrationsError> {
    conn.transaction(|| {
        try!(writeln!(
            output,
            "Rolling back migration {}",
            migration.version()
        ));
        try!(migration.revert(conn));
        let target = __diesel_schema_migrations.filter(version.eq(migration.version()));
        try!(::diesel::delete(target).execute(conn));
        Ok(())
    })
}

/// Returns the directory containing migrations. Will look at for
/// $PWD/migrations. If it is not found, it will search the parents of the
/// current directory, until it reaches the root directory.  Returns
/// `MigrationError::MigrationDirectoryNotFound` if no directory is found.
pub fn find_migrations_directory() -> Result<PathBuf, MigrationError> {
    search_for_migrations_directory(&try!(env::current_dir()))
}

/// Searches for the migrations directory relative to the given path. See
/// `find_migrations_directory` for more details.
pub fn search_for_migrations_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    let migration_path = path.join("migrations");
    if migration_path.is_dir() {
        Ok(migration_path)
    } else {
        path.parent()
            .map(search_for_migrations_directory)
            .unwrap_or(Err(MigrationError::MigrationDirectoryNotFound))
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::*;

    use self::tempdir::TempDir;
    use std::fs;

    #[test]
    fn migration_directory_not_found_if_no_migration_dir_exists() {
        let dir = TempDir::new("diesel").unwrap();

        assert_eq!(
            Err(MigrationError::MigrationDirectoryNotFound),
            search_for_migrations_directory(dir.path())
        );
    }

    #[test]
    fn migration_directory_defaults_to_pwd_slash_migrations() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");

        fs::create_dir(&migrations_path).unwrap();

        assert_eq!(
            Ok(migrations_path),
            search_for_migrations_directory(&temp_path)
        );
    }

    #[test]
    fn migration_directory_checks_parents() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");
        let child_path = temp_path.join("child");

        fs::create_dir(&child_path).unwrap();
        fs::create_dir(&migrations_path).unwrap();

        assert_eq!(
            Ok(migrations_path),
            search_for_migrations_directory(&child_path)
        );
    }
}
