use diesel::result;

use std::convert::From;
use std::error::Error;
use std::{fmt, io};

use self::DatabaseError::*;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Debug)]
pub enum DatabaseError {
    ProjectRootNotFound,
    DatabaseUrlMissing,
    IoError(io::Error),
    QueryError(result::Error),
    ConnectionError(result::ConnectionError),
}

impl From<io::Error> for DatabaseError {
    fn from(e: io::Error) -> Self {
        IoError(e)
    }
}

impl From<result::Error> for DatabaseError {
    fn from(e: result::Error) -> Self {
        QueryError(e)
    }
}

impl From<result::ConnectionError> for DatabaseError {
    fn from(e: result::ConnectionError) -> Self {
        ConnectionError(e)
    }
}

impl Error for DatabaseError {}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ProjectRootNotFound => {
                f.write_str("Unable to find diesel.toml or Cargo.toml in this directory or any parent directories.")
            }
            DatabaseUrlMissing => {
                f.write_str("The --database-url argument must be passed, or the DATABASE_URL environment variable must be set.")
            }
            IoError(ref error) => f.write_str(&error
                .source()
                .map(ToString::to_string)
                .unwrap_or_else(|| error.to_string())),
            QueryError(ref error) => f.write_str(&error
                .source()
                .map(ToString::to_string)
                .unwrap_or_else(|| error.to_string())),
            ConnectionError(ref error) => f.write_str(&error
                .source()
                .map(ToString::to_string)
                .unwrap_or_else(|| error.to_string())),
        }
    }
}

impl PartialEq for DatabaseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&ProjectRootNotFound, &ProjectRootNotFound) => true,
            _ => false,
        }
    }
}
