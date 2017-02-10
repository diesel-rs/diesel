use result::{self, TransactionError};

use std::convert::From;
use std::{fmt, io};
use std::path::PathBuf;
use std::error::Error;

#[derive(Debug)]
pub enum MigrationError {
    MigrationDirectoryNotFound,
    UnknownMigrationFormat(PathBuf),
    IoError(io::Error),
    UnknownMigrationVersion(String),
}

impl Error for MigrationError {
    fn description(&self) -> &str {
        match *self {
            MigrationError::MigrationDirectoryNotFound =>
                "Unable to find migrations directory in this directory or any parent directories.",
            MigrationError::UnknownMigrationFormat(_) =>
                "Invalid migration directory, the directory's name should be \
                <timestamp>_<name_of_migration>, and it should only contain up.sql and down.sql.",
            MigrationError::IoError(ref error) =>
                error.description(),
            MigrationError::UnknownMigrationVersion(_) =>
                "Unable to find migration version to revert in the migrations directory.",
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
            _ => false
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(e: io::Error) -> Self {
        MigrationError::IoError(e)
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "clippy", allow(enum_variant_names))]
pub enum RunMigrationsError {
    MigrationError(MigrationError),
    QueryError(result::Error),
}

impl Error for RunMigrationsError {
    fn description(&self) -> &str {
        match *self {
            RunMigrationsError::MigrationError(ref error) => error.description(),
            RunMigrationsError::QueryError(ref error) => error.description(),
        }
    }
}

impl fmt::Display for RunMigrationsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.description().fmt(f)
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

impl From<TransactionError<RunMigrationsError>> for RunMigrationsError {
    fn from(e: TransactionError<RunMigrationsError>) -> Self {
        use result::TransactionError::*;
        match e {
            CouldntCreateTransaction(e) => RunMigrationsError::from(e),
            UserReturnedError(e) => e,
        }
    }
}
