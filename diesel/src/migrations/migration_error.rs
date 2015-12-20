use result;

use std::convert::From;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum MigrationError {
    MigrationDirectoryNotFound,
    UnknownMigrationFormat(PathBuf),
    IoError(io::Error),
}

impl PartialEq for MigrationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                &MigrationError::MigrationDirectoryNotFound,
                &MigrationError::MigrationDirectoryNotFound,
            ) => true,
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
pub enum RunMigrationsError {
    MigrationError(MigrationError),
    QueryError(result::Error),
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
