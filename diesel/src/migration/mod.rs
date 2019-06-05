//! Representation of migrations

mod errors;
pub use self::errors::{MigrationError, RunMigrationsError};

use connection::SimpleConnection;
use std::any::Any;
use std::path::Path;

/// Represents a migration that interacts with diesel
pub trait Migration {
    /// Get the migration version
    fn version(&self) -> &str;
    /// Apply this migration
    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Revert this migration
    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Get the migration file path
    fn file_path(&self) -> Option<&Path> {
        None
    }
    /// Get the metadata associated with this migration, if any
    fn metadata(&self) -> Option<&Metadata> {
        None
    }
}

impl Migration for Box<dyn Migration> {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
    }

    fn metadata(&self) -> Option<&Metadata> {
        (&**self).metadata()
    }
}

impl<'a> Migration for &'a dyn Migration {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
    }

    fn metadata(&self) -> Option<&Metadata> {
        (&**self).metadata()
    }
}

/// Represents metadata associated with a migration.
///
/// The format of a migration's metadata is dependent on the migration format
/// being used.
///
/// For Diesel's built in SQL file migrations, metadata is stored in a file
/// called `metadata.toml`. Diesel looks for a single key, `run_in_transaction`.
/// By default, all migrations are run in a transaction on SQLite and
/// PostgreSQL. This behavior can be disabled for a single migration by setting
/// this to `false`.
pub trait Metadata {
    /// Get the metadata at the given key, if present
    fn get(&self, key: &str) -> Option<&dyn Any>;
}
