use diesel::result;

use std::convert::From;
use std::error::Error;
use std::{fmt, io};

use self::DatabaseError::*;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Debug)]
pub enum DatabaseError {
    CargoTomlNotFound,
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

impl Error for DatabaseError {
    fn description(&self) -> &str {
        match *self {
            CargoTomlNotFound => {
                "Unable to find Cargo.toml in this directory or any parent directories."
            }
            DatabaseUrlMissing => {
                "The --database-url argument must be passed, or the DATABASE_URL environment variable must be set."
            }
            IoError(ref error) => error
                .source()
                .map(Error::description)
                .unwrap_or_else(|| error.description()),
            QueryError(ref error) => error
                .source()
                .map(Error::description)
                .unwrap_or_else(|| error.description()),
            ConnectionError(ref error) => error
                .source()
                .map(Error::description)
                .unwrap_or_else(|| error.description()),
        }
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.description().fmt(f)
    }
}

impl PartialEq for DatabaseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&CargoTomlNotFound, &CargoTomlNotFound) => true,
            _ => false,
        }
    }
}
