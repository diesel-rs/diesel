//! Error types that represent migration errors.
//! These are split into multiple segments, depending on
//! where in the migration process an error occurs.

use std::error::Error;
use std::path::PathBuf;
use std::{fmt, io};

use diesel::migration::MigrationVersion;

use crate::file_based_migrations::DieselMigrationName;

/// Errors that occur while preparing to run migrations
#[derive(Debug)]
#[non_exhaustive]
pub enum MigrationError {
    /// The migration directory wasn't found
    MigrationDirectoryNotFound(PathBuf),
    /// Provided migration was in an unknown format
    UnknownMigrationFormat(PathBuf),
    /// General system IO error
    IoError(io::Error),
    /// Provided migration had an incompatible version number
    UnknownMigrationVersion(MigrationVersion<'static>),
    /// No migrations had to be/ could be run
    NoMigrationRun,
    /// `down.sql` file is missing, and you're asking for a `revert`
    /// or `redo`
    NoMigrationRevertFile,
}

impl Error for MigrationError {}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            MigrationError::MigrationDirectoryNotFound(ref p) => write!(
                f,
                "Unable to find migrations directory in {p:?} or any parent directories."
            ),
            MigrationError::UnknownMigrationFormat(_) => write!(
                f,
                "Invalid migration directory: the directory's name should be \
                 <timestamp>_<name_of_migration>, and it should contain up.sql and \
                 optionally down.sql."
            ),
            MigrationError::IoError(ref error) => write!(f, "{error}"),
            MigrationError::UnknownMigrationVersion(ref version) => write!(
                f,
                "Unable to find migration version {version} to revert in the migrations directory."
            ),
            MigrationError::NoMigrationRun => write!(
                f,
                "No migrations have been run. Did you forget `diesel migration run`?"
            ),
            MigrationError::NoMigrationRevertFile => {
                write!(f, "Missing `down.sql` file to revert migration")
            }
        }
    }
}

impl PartialEq for MigrationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                &MigrationError::MigrationDirectoryNotFound(_),
                &MigrationError::MigrationDirectoryNotFound(_),
            ) => true,
            (
                MigrationError::UnknownMigrationFormat(p1),
                MigrationError::UnknownMigrationFormat(p2),
            ) => p1 == p2,
            _ => false,
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(e: io::Error) -> Self {
        MigrationError::IoError(e)
    }
}

/// Errors that occur while running migrations
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum RunMigrationsError {
    /// A general migration error occurred
    MigrationError(DieselMigrationName, MigrationError),
    /// The provided migration included an invalid query
    QueryError(DieselMigrationName, diesel::result::Error),
    /// The provided migration was empty
    EmptyMigration(DieselMigrationName),
}

impl Error for RunMigrationsError {}

impl fmt::Display for RunMigrationsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RunMigrationsError::MigrationError(v, err) => {
                write!(f, "Failed to run {v} with: {err}")
            }
            RunMigrationsError::QueryError(v, err) => {
                write!(f, "Failed to run {v} with: {err}")
            }
            RunMigrationsError::EmptyMigration(v) => write!(
                f,
                "Failed to run {v} with: Attempted to run an empty migration."
            ),
        }
    }
}
