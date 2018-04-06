//! Representation of migrations

mod errors;
mod annotation;

pub use self::errors::{MigrationError, RunMigrationsError};
pub use self::annotation::{MigrationAnnotation, AnnotatedMigration};

use connection::SimpleConnection;
use std::path::Path;
use std::fmt::{Debug, Display};
use proc_macro::TokenStream;

/// Migration source
pub trait MigrationSource: Debug {
    /// The type of each record returned by the migration source.
    /// May be a `Box<Migration>`.
    type MigrationEntry: Migration;
    /// Takes a snapshot of the migrations provided by this source
    fn list_migrations(&self) -> Vec<Self::MigrationEntry>;
}

/// This is the entry-point for extensions to diesel's migration system
/// It's not necessary to use this if you implement your own `MigrationSource`
pub trait MigrationPlugin: Debug {
    /// Attempt to load a `Migration` from a path using this plugin.
    /// Returns `UnknownMigrationFormat` if the plugin doesn't provide this type of migration.
    fn load_migration_from_path(&self, path: &Path) -> Result<Box<Migration>, MigrationError> {
        Err(MigrationError::UnknownMigrationFormat(path.into()))
    }
    /// Load any annotations provided by this plugin from a path, and attach them
    /// to a migration. Returns `Ok(())` even if no annotations were found.
    fn load_annotations_from_path(&self, _path: &Path, _migration: &mut AnnotatedMigration) -> Result<(), MigrationError> {
        Ok(())
    }
}

/// Represents a migration that interacts with diesel
pub trait Migration: Debug + Display {
    /// Get the migration version
    fn version(&self) -> &str;

    /// Apply this migration
    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;

    /// Revert this migration
    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;

    /// Get the migration file path, if applicable
    fn file_path(&self) -> Option<&Path> {
        None
    }

    /// Should this migration be run in a transaction?
    fn needs_transaction(&self) -> bool {
        true
    }

    /// Embed this migration
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        Err(MigrationError::MigrationNotEmbeddable)
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
    fn needs_transaction(&self) -> bool {
        (&**self).needs_transaction()
    }
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        (&**self).embed()
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
    fn needs_transaction(&self) -> bool {
        (&**self).needs_transaction()
    }
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        (&**self).embed()
    }
}
