extern crate postgres;

use std::result;
use std::convert::From;

#[derive(Debug)]
pub enum Error {
    NativeError(postgres::error::Error),
}

#[derive(Debug)]
pub enum ConnectionError {
    NativeError(postgres::error::ConnectError),
}

#[derive(Debug, PartialEq)]
pub enum FindError {
    RecordNotFound,
    Error(Error),
}

pub type Result<T> = result::Result<T, Error>;
pub type ConnectionResult<T> = result::Result<T, ConnectionError>;

impl From<postgres::error::Error> for Error {
    fn from(e: postgres::error::Error) -> Self {
        Error::NativeError(e)
    }
}

impl From<postgres::error::ConnectError> for ConnectionError {
    fn from(e: postgres::error::ConnectError) -> Self {
        ConnectionError::NativeError(e)
    }
}

// PG doesn't implement PartialEq for its error types
impl PartialEq for Error {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
