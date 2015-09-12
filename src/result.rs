use std::result;
use std::convert::From;
use std::ffi::NulError;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidCString(NulError),
    DatabaseError(String),
}

#[derive(Debug)]
pub enum ConnectionError {
    InvalidCString(NulError),
    BadConnection(String),
}

pub type Result<T> = result::Result<T, Error>;
pub type ConnectionResult<T> = result::Result<T, ConnectionError>;

impl From<NulError> for ConnectionError {
    fn from(e: NulError) -> Self {
        ConnectionError::InvalidCString(e)
    }
}

impl From<NulError> for Error {
    fn from(e: NulError) -> Self {
        Error::InvalidCString(e)
    }
}
