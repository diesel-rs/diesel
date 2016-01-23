use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{self, Display, Write};
use std::ffi::NulError;

#[derive(Debug)]
/// The generic "things can fail in a myriad of ways" enum. This type is not
/// indended to be exhaustively matched, and new variants may be added in the
/// future without a major version bump.
pub enum Error {
    InvalidCString(NulError),
    DatabaseError(String),
    NotFound,
    QueryBuilderError(Box<StdError>),
    #[doc(hidden)]
    __Nonexhaustive,
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

pub type QueryResult<T> = Result<T, Error>;
pub type ConnectionResult<T> = Result<T, ConnectionError>;
pub type TransactionResult<T, E> = Result<T, TransactionError<E>>;

pub trait OptionalExtension<T> {
    fn optional(self) -> Result<Option<T>, Error>;
}

impl<T> OptionalExtension<T> for QueryResult<T> {
    fn optional(self) -> Result<Option<T>, Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

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

impl From<TransactionError<Error>> for Error {
    fn from(e: TransactionError<Error>) -> Self {
        match e {
            TransactionError::CouldntCreateTransaction(e) => e,
            TransactionError::UserReturnedError(e) => e,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidCString(ref nul_err) => nul_err.fmt(f),
            &Error::DatabaseError(ref s) => write!(f, "{}", &s),
            &Error::NotFound => f.write_str("NotFound"),
            &Error::QueryBuilderError(ref e) => e.fmt(f),
            &Error::__Nonexhaustive => unreachable!(),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            &Error::InvalidCString(ref nul_err) => nul_err.description(),
            &Error::DatabaseError(ref s) => &s,
            &Error::NotFound => "Record not found",
            &Error::QueryBuilderError(ref e) => e.description(),
            &Error::__Nonexhaustive => unreachable!(),
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

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::InvalidCString(ref a), &Error::InvalidCString(ref b)) => a == b,
            (&Error::DatabaseError(ref a), &Error::DatabaseError(ref b)) => a == b,
            (&Error::NotFound, &Error::NotFound) => true,
            _ => false,
        }
    }
}
