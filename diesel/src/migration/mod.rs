//! Representation of migrations

mod errors;
pub use self::errors::{MigrationError, RunMigrationsError};

use connection::SimpleConnection;
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
}
