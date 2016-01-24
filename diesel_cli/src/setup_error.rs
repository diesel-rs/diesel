use diesel::result;

use std::convert::From;
use std::{fmt, io};
use std::error::Error;

use self::SetupError::*;

#[derive(Debug)]
pub enum SetupError {
    #[allow(dead_code)]
    CargoTomlNotFound,
    IoError(io::Error),
    QueryError(result::Error),
    ConnectionError(result::ConnectionError),
}

impl From<io::Error> for SetupError {
    fn from(e: io::Error) -> Self {
        IoError(e)
    }
}

impl From<result::Error> for SetupError {
    fn from(e: result::Error) -> Self {
        QueryError(e)
    }
}

impl From<result::ConnectionError> for SetupError {
    fn from(e: result::ConnectionError) -> Self {
        ConnectionError(e)
    }
}

impl Error for SetupError {
    fn description(&self) -> &str {
        match *self {
            CargoTomlNotFound => "Unable to find Cargo.toml in this directory or any parent directories.",
            IoError(ref error) => error.cause().map(|e| e.description()).unwrap_or(error.description()),
            QueryError(ref error) => error.cause().map(|e| e.description()).unwrap_or(error.description()),
            ConnectionError(ref error) => error.cause().map(|e| e.description()).unwrap_or(error.description()),
        }
    }
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.description().fmt(f)
    }
}

impl PartialEq for SetupError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                &CargoTomlNotFound,
                &CargoTomlNotFound,
            ) => true,
            _ => false
        }
    }
}
