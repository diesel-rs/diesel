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

#[derive(Debug, PartialEq)]
pub enum TransactionError<E> {
    CouldntCreateTransaction(Error),
    UserReturnedError(E),
}

pub type Result<T> = result::Result<T, Error>;
pub type ConnectionResult<T> = result::Result<T, ConnectionError>;
pub type TransactionResult<T, E> = result::Result<T, TransactionError<E>>;

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

impl<E> From<Error> for TransactionError<E> {
    fn from(e: Error) -> Self {
        TransactionError::CouldntCreateTransaction(e)
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

impl<E: Display> Display for TransactionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &TransactionError::CouldntCreateTransaction(ref e) => e.fmt(f),
            &TransactionError::UserReturnedError(ref e) => e.fmt(f),
        }
    }
}

impl<E: StdError> StdError for TransactionError<E> {
    fn description(&self) -> &str {
        match self {
            &TransactionError::CouldntCreateTransaction(ref e) => e.description(),
            &TransactionError::UserReturnedError(ref e) => e.description(),
        }
    }
}
