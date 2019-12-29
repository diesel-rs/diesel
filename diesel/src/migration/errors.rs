//! Error types that represent migration errors.
//! These are split into multiple segments, depending on
//! where in the migration process an error occurs.

use std::convert::From;
use std::error::Error;
use std::path::PathBuf;
use std::{fmt, io};

use crate::result;

/// Errors that occur while preparing to run migrations
#[derive(Debug)]
pub enum MigrationError {
    /// The migration directory wasn't found
    MigrationDirectoryNotFound,
    /// Provided migration was in an unknown format
    UnknownMigrationFormat(PathBuf),
    /// General system IO error
    IoError(io::Error),
    /// Provided migration had an incompatible version number
    UnknownMigrationVersion(String),
    /// No migrations had to be/ could be run
    NoMigrationRun,
    ///
    #[doc(hidden)]
    __NonExhaustive,
}

impl Error for MigrationError {
    fn description(&self) -> &str {
        match *self {
            MigrationError::MigrationDirectoryNotFound => {
                "Unable to find migrations directory in this directory or any parent directories."
            }
            MigrationError::UnknownMigrationFormat(_) => {
                "Invalid migration directory, the directory's name should be \
                 <timestamp>_<name_of_migration>, and it should only contain up.sql and down.sql."
            }
            MigrationError::IoError(ref error) => error.description(),
            MigrationError::UnknownMigrationVersion(_) => {
                "Unable to find migration version to revert in the migrations directory."
            }
            MigrationError::NoMigrationRun => {
                "No migrations have been run. Did you forget `diesel migration run`?"
            }
            MigrationError::__NonExhaustive => unreachable!(),
        }
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.description().fmt(f)
    }
}

impl PartialEq for MigrationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                &MigrationError::MigrationDirectoryNotFound,
                &MigrationError::MigrationDirectoryNotFound,
            ) => true,
            (
                &MigrationError::UnknownMigrationFormat(ref p1),
                &MigrationError::UnknownMigrationFormat(ref p2),
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
pub enum RunMigrationsError {
    /// A general migration error occured
    MigrationError(MigrationError),
    /// The provided migration included an invalid query
    QueryError(result::Error),
    /// The provided migration was empty
    EmptyMigration,
    ///
    #[doc(hidden)]
    __NonExhaustive,
}

impl Error for RunMigrationsError {
    fn description(&self) -> &str {
        match *self {
            RunMigrationsError::MigrationError(ref error) => error.description(),
            RunMigrationsError::QueryError(ref error) => error.description(),
            RunMigrationsError::EmptyMigration => "Attempted to run an empty migration.",
            RunMigrationsError::__NonExhaustive => unreachable!(),
        }
    }
}

impl fmt::Display for RunMigrationsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "Failed with: {}", self.description())
    }
}

impl From<MigrationError> for RunMigrationsError {
    fn from(e: MigrationError) -> Self {
        RunMigrationsError::MigrationError(e)
    }
}

impl From<result::Error> for RunMigrationsError {
    fn from(e: result::Error) -> Self {
        RunMigrationsError::QueryError(e)
    }
}

impl From<io::Error> for RunMigrationsError {
    fn from(e: io::Error) -> Self {
        RunMigrationsError::MigrationError(e.into())
    }
}
