//!

pub mod errors;
pub use self::errors::*;

use connection::SimpleConnection;
use std::path::Path;
use std::fmt;

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

/// A migration name
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct MigrationName<'a> {
    /// Wraps around a migration
    pub migration: &'a Migration,
}

/// Get the name of a migration
pub fn name(migration: &Migration) -> MigrationName {
    MigrationName {
        migration: migration,
    }
}

impl<'a> fmt::Display for MigrationName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let file_name = self.migration
            .file_path()
            .and_then(|file_path| file_path.file_name())
            .and_then(|file| file.to_str());
        if let Some(name) = file_name {
            f.write_str(name)?;
        } else {
            f.write_str(self.migration.version())?;
        }
        Ok(())
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
