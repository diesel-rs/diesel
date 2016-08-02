use result::{self, TransactionError};

use std::convert::From;
use std::io;
use std::path::PathBuf;

quick_error! {
    #[derive(Debug)]
    pub enum MigrationError {
        MigrationDirectoryNotFound {
            description("Unable to find migrations directory in this directory or any parent directories.")
        }
        UnknownMigrationFormat(path: PathBuf) {
            description("Invalid migration directory, the directory's name should be <timestamp>_<name_of_migration>, and it should only contain up.sql and down.sql.")
        }
        IoError(error: io::Error) {
            from()
            description(error.description())
        }
        UnknownMigrationVersion(message: String) {
            description("Unable to find migration version to revert in the migrations directory.")
        }
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

quick_error! {
    #[derive(Debug)]
    pub enum RunMigrationsError {
        MigrationError(error: MigrationError) {
            from()
            description(error.description())
        }
        QueryError(error: result::Error) {
            description(error.description())
        }
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
