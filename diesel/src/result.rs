use std::convert::From;
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::ffi::NulError;

#[derive(Debug)]
#[cfg_attr(feature = "clippy", allow(enum_variant_names))]
/// The generic "things can fail in a myriad of ways" enum. This type is not
/// intended to be exhaustively matched, and new variants may be added in the
/// future without a major version bump.
pub enum Error {
    InvalidCString(NulError),
    DatabaseError(
        DatabaseErrorKind,
        Box<DatabaseErrorInformation + Send + Sync>,
    ),
    NotFound,
    QueryBuilderError(Box<StdError + Send + Sync>),
    DeserializationError(Box<StdError + Send + Sync>),
    SerializationError(Box<StdError + Send + Sync>),
    /// You can return this variant inside of a transaction when you want to
    /// roll it back, but have no actual error to return. Diesel will never
    /// return this variant unless you gave it to us, and it can be safely
    /// ignored in error handling.
    RollbackTransaction,
    #[doc(hidden)] __Nonexhaustive,
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
    #[doc(hidden)] __Unknown, // Match against _ instead, more variants may be added in the future
}

pub trait DatabaseErrorInformation {
    fn message(&self) -> &str;
    fn details(&self) -> Option<&str>;
    fn hint(&self) -> Option<&str>;
    fn table_name(&self) -> Option<&str>;
    fn column_name(&self) -> Option<&str>;
    fn constraint_name(&self) -> Option<&str>;
}

impl fmt::Debug for DatabaseErrorInformation + Send + Sync {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.message(), f)
    }
}

impl DatabaseErrorInformation for String {
    fn message(&self) -> &str {
        self
    }

    fn details(&self) -> Option<&str> {
        None
    }
    fn hint(&self) -> Option<&str> {
        None
    }
    fn table_name(&self) -> Option<&str> {
        None
    }
    fn column_name(&self) -> Option<&str> {
        None
    }
    fn constraint_name(&self) -> Option<&str> {
        None
    }
}

/// Errors which can occur during [`Connection::establish`]
///
/// [`Connection::establish`]: ../connection/trait.Connection.html?search=#tymethod.establish
#[derive(Debug)]
pub enum ConnectionError {
    /// The connection URL contained a `NUL` byte.
    InvalidCString(NulError),
    /// The database returned an error.
    BadConnection(String),
    /// The connection URL could not be parsed.
    InvalidConnectionUrl(String),
    /// Diesel could not configure the database connection.
    ///
    /// Diesel may try to automatically set session specific configuration
    /// values, such as UTF8 encoding, or enabling the `||` operator on MySQL.
    /// This variant is returned if an error occurred executing the query to set
    /// those options. Diesel will never affect global configuration.
    CouldntSetupConfiguration(Error),
    #[doc(hidden)] __Nonexhaustive, // Match against _ instead, more variants may be added in the future
}

/// A specialized result type for queries.
///
/// This type is exported by `diesel::prelude`, and is generally used by any
/// code which is interacting with Diesel. This type exists to avoid writing out
/// `diesel::result::Error`, and is otherwise a direct mapping to `Result`.
pub type QueryResult<T> = Result<T, Error>;

/// A specialized result type for establishing connections.
///
/// This type exists to avoid writing out `diesel::result::ConnectionError`, and
/// is otherwise a direct mapping to `Result`.
pub type ConnectionResult<T> = Result<T, ConnectionError>;

/// See the [method documentation](#tymethod.optional).
pub trait OptionalExtension<T> {
    /// Converts a `QueryResult<T>` into a `QueryResult<Option<T>>`.
    ///
    /// By default, Diesel treats 0 rows being returned from a query that is expected to return 1
    /// row as an error (e.g. the return value of [`get_result`] or [`first`]). This method will
    /// handle that error, and give you back an `Option<T>` instead.
    ///
    /// [`get_result`]: ../prelude/trait.LoadDsl.html#method.get_result
    /// [`first`]: ../prelude/trait.FirstDsl.html#method.first
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::result::{QueryResult, Error};
    /// use diesel::OptionalExtension;
    ///
    /// let result: QueryResult<i32> = Ok(1);
    /// assert_eq!(Ok(Some(1)), result.optional());
    ///
    /// let result: QueryResult<i32> = Err(Error::NotFound);
    /// assert_eq!(Ok(None), result.optional());
    /// ```
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
            (&Error::DatabaseError(_, ref a), &Error::DatabaseError(_, ref b)) => {
                a.message() == b.message()
            }
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

pub(crate) fn first_or_not_found<T>(records: QueryResult<Vec<T>>) -> QueryResult<T> {
    records?.into_iter().next().ok_or(Error::NotFound)
}
