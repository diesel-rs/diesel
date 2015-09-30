use std::result;
use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{self, Display, Write};
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

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidCString(ref nul_err) => nul_err.fmt(f),
            &Error::DatabaseError(ref s) => write!(f, "{}", &s),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            &Error::InvalidCString(ref nul_err) => nul_err.description(),
            &Error::DatabaseError(ref s) => &s,
        }
    }
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ConnectionError::InvalidCString(ref nul_err) => nul_err.fmt(f),
            &ConnectionError::BadConnection(ref s) => write!(f, "{}", &s),
        }
    }
}

impl StdError for ConnectionError {
    fn description(&self) -> &str {
        match self {
            &ConnectionError::InvalidCString(ref nul_err) => nul_err.description(),
            &ConnectionError::BadConnection(ref s) => &s,
        }
    }
}
