use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::ffi::NulError;

#[derive(Debug)]
#[cfg_attr(feature = "clippy", allow(enum_variant_names))]
/// The generic "things can fail in a myriad of ways" enum. This type is not
/// indended to be exhaustively matched, and new variants may be added in the
/// future without a major version bump.
pub enum Error {
    InvalidCString(NulError),
    DatabaseError(DatabaseErrorKind, Box<DatabaseErrorInformation+Send+Sync>),
    NotFound,
    QueryBuilderError(Box<StdError+Send+Sync>),
    DeserializationError(Box<StdError+Send+Sync>),
    SerializationError(Box<StdError+Send+Sync>),
    /// You can return this variant inside of a transaction when you want to
    /// roll it back, but have no actual error to return. Diesel will never
    /// return this variant unless you gave it to us, and it can be safely
    /// ignored in error handling.
    RollbackTransaction,
    #[doc(hidden)]
    __Nonexhaustive,
}

#[derive(Debug, Clone, Copy)]
/// The kind of database error that occurred. This is not meant to exhaustively
/// cover all possible errors, but is used to identify errors which are commonly
/// recovered from programatically. This enum is not intended to be exhaustively
/// matched, and new variants may be added in the future without a major version
/// bump.
pub enum DatabaseErrorKind {
    UniqueViolation,
    ForeignKeyViolation,
    UnableToSendCommand,
    #[doc(hidden)]
    __Unknown, // Match against _ instead, more variants may be added in the future
}

pub trait DatabaseErrorInformation {
    fn message(&self) -> &str;
    fn details(&self) -> Option<&str>;
    fn hint(&self) -> Option<&str>;
    fn table_name(&self) -> Option<&str>;
    fn column_name(&self) -> Option<&str>;
    fn constraint_name(&self) -> Option<&str>;
}

impl fmt::Debug for DatabaseErrorInformation+Send+Sync {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.message(), f)
    }
}

impl DatabaseErrorInformation for String {
    fn message(&self) -> &str {
        self
    }

    fn details(&self) -> Option<&str> { None }
    fn hint(&self) -> Option<&str> { None }
    fn table_name(&self) -> Option<&str> { None }
    fn column_name(&self) -> Option<&str> { None }
    fn constraint_name(&self) -> Option<&str> { None }
}

#[derive(Debug)]
pub enum ConnectionError {
    InvalidCString(NulError),
    BadConnection(String),
    InvalidConnectionUrl(String),
    /// Diesel may try to automatically set session specific configuration
    /// values, such as UTF8 encoding, or enabling the `||` operator on MySQL.
    /// This variant is returned if an error occurred executing the query to set
    /// those options. Diesel will never affect global configuration.
    CouldntSetupConfiguration(Error),
    #[doc(hidden)]
    __Nonexhaustive, // Match against _ instead, more variants may be added in the future
}

pub type QueryResult<T> = Result<T, Error>;
pub type ConnectionResult<T> = Result<T, ConnectionError>;

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

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidCString(ref nul_err) => nul_err.fmt(f),
            Error::DatabaseError(_, ref e) => write!(f, "{}", e.message()),
            Error::NotFound => f.write_str("NotFound"),
            Error::QueryBuilderError(ref e) => e.fmt(f),
            Error::DeserializationError(ref e) => e.fmt(f),
            Error::SerializationError(ref e) => e.fmt(f),
            Error::RollbackTransaction => write!(f, "{}", self.description()),
            Error::__Nonexhaustive => unreachable!(),
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidCString(ref nul_err) => nul_err.description(),
            Error::DatabaseError(_, ref e) => e.message(),
            Error::NotFound => "Record not found",
            Error::QueryBuilderError(ref e) => e.description(),
            Error::DeserializationError(ref e) => e.description(),
            Error::SerializationError(ref e) => e.description(),
            Error::RollbackTransaction => "The current transaction was aborted",
            Error::__Nonexhaustive => unreachable!(),
        }
    }
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConnectionError::InvalidCString(ref nul_err) => nul_err.fmt(f),
            ConnectionError::BadConnection(ref s) => write!(f, "{}", s),
            ConnectionError::InvalidConnectionUrl(ref s) => write!(f, "{}", s),
            ConnectionError::CouldntSetupConfiguration(ref e) => e.fmt(f),
            ConnectionError::__Nonexhaustive => unreachable!(),
        }
    }
}

impl StdError for ConnectionError {
    fn description(&self) -> &str {
        match *self {
            ConnectionError::InvalidCString(ref nul_err) => nul_err.description(),
            ConnectionError::BadConnection(ref s) => s,
            ConnectionError::InvalidConnectionUrl(ref s) => s,
            ConnectionError::CouldntSetupConfiguration(ref e) => e.description(),
            ConnectionError::__Nonexhaustive => unreachable!(),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::InvalidCString(ref a), &Error::InvalidCString(ref b)) => a == b,
            (&Error::DatabaseError(_, ref a), &Error::DatabaseError(_, ref b)) =>
                a.message() == b.message(),
            (&Error::NotFound, &Error::NotFound) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
#[allow(warnings)]
fn error_impls_send() {
    let err: Error = unimplemented!();
    let x: &Send = &err;
}
