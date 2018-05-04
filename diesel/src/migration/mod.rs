//! Representation of migrations

mod errors;
pub use self::errors::{MigrationError, RunMigrationsError};

use connection::{Connection, SimpleConnection};
use std::path::Path;

/// Represents a migration that interacts with diesel
pub trait Migration {
    /// Get the migration version
    fn version(&self) -> &str;
    /// Apply this migration
    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Revert this migration
    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Get the migration file path
    fn file_path(&self) -> Option<&Path> {
        None
    }
}

impl Migration for Box<Migration> {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
    }
}

impl<'a> Migration for &'a Migration {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
    }
}

/// A trait indicating that a connection could be used to run migrations
///
/// Normal users of diesel should not use/see this trait.
/// This trait is only relevant when you are implementing a new connection type
/// that could be used with diesel.
pub trait MigrationConnection: Connection {
    /// The create table statement used to create the internal table used to
    /// track which migrations were already run
    ///
    /// This constant should contain a `CREATE TABLE` statement that
    /// creates a new table called `__diesel_schema_migrations` containing
    /// two columns named `version` and `run_on`. The `version` column must have a
    /// datatype compatible with diesels `Text` sql type and must be the primary key
    /// of the table. The `run_on` column must have a datatype compatible with diesels
    /// `Timestamp` sql type, have a `NOT NULL` annotation and a default value
    /// corresponding to the actual insert time (`CURRENT_TIMESTAMP` in ISO sql).
    const CREATE_MIGRATIONS_TABLE: &'static str =
        "CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (\
         version VARCHAR(50) PRIMARY KEY NOT NULL,\
         run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP\
         )";
}
