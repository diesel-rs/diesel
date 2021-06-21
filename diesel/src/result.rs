//! Errors, type aliases, and functions related to working with `Result`.

use std::convert::From;
use std::error::Error as StdError;
use std::ffi::NulError;
use std::fmt::{self, Display};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
/// Represents all the ways that a query can fail.
///
/// This type is not intended to be exhaustively matched, and new variants may
/// be added in the future without a major version bump.
#[non_exhaustive]
pub enum Error {
    /// The query contained a nul byte.
    ///
    /// This should never occur in normal usage.
    InvalidCString(NulError),

    /// The database returned an error.
    ///
    /// While Diesel prevents almost all sources of runtime errors at compile
    /// time, it does not attempt to prevent 100% of them. Typically this error
    /// will occur from insert or update statements due to a constraint
    /// violation.
    DatabaseError(
        DatabaseErrorKind,
        Box<dyn DatabaseErrorInformation + Send + Sync>,
    ),

    /// No rows were returned by a query expected to return at least one row.
    ///
    /// This variant is only returned by [`get_result`] and [`first`]. [`load`]
    /// does not treat 0 rows as an error. If you would like to allow either 0
    /// or 1 rows, call [`optional`] on the result.
    ///
    /// [`get_result`]: crate::query_dsl::RunQueryDsl::get_result()
    /// [`first`]: crate::query_dsl::RunQueryDsl::first()
    /// [`load`]: crate::query_dsl::RunQueryDsl::load()
    /// [`optional`]: OptionalExtension::optional
    NotFound,

    /// The query could not be constructed
    ///
    /// An example of when this error could occur is if you are attempting to
    /// construct an update statement with no changes (e.g. all fields on the
    /// struct are `None`).
    QueryBuilderError(Box<dyn StdError + Send + Sync>),

    /// An error occurred deserializing the data being sent to the database.
    ///
    /// Typically this error means that the stated type of the query is
    /// incorrect. An example of when this error might occur in normal usage is
    /// attempting to deserialize an infinite date into chrono.
    DeserializationError(Box<dyn StdError + Send + Sync>),

    /// An error occurred serializing the data being sent to the database.
    ///
    /// An example of when this error would be returned is if you attempted to
    /// serialize a `chrono::NaiveDate` earlier than the earliest date supported
    /// by PostgreSQL.
    SerializationError(Box<dyn StdError + Send + Sync>),

    /// An error occurred during the rollback of a transaction.
    ///
    /// An example of when this error would be returned is if a rollback has
    /// already be called on the current transaction.
    RollbackError(Box<Error>),

    /// Roll back the current transaction.
    ///
    /// You can return this variant inside of a transaction when you want to
    /// roll it back, but have no actual error to return. Diesel will never
    /// return this variant unless you gave it to us, and it can be safely
    /// ignored in error handling.
    RollbackTransaction,

    /// Attempted to perform an operation that cannot be done inside a transaction
    /// when a transaction was already open.
    AlreadyInTransaction,
}

#[derive(Debug, Clone, Copy)]
/// The kind of database error that occurred.
///
/// This is not meant to exhaustively cover all possible errors, but is used to
/// identify errors which are commonly recovered from programmatically. This enum
/// is not intended to be exhaustively matched, and new variants may be added in
/// the future without a major version bump.
#[non_exhaustive]
pub enum DatabaseErrorKind {
    /// A unique constraint was violated.
    UniqueViolation,

    /// A foreign key constraint was violated.
    ForeignKeyViolation,

    /// The query could not be sent to the database due to a protocol violation.
    ///
    /// An example of a case where this would occur is if you attempted to send
    /// a query with more than 65000 bind parameters using PostgreSQL.
    UnableToSendCommand,

    /// A serializable transaction failed to commit due to a read/write
    /// dependency on a concurrent transaction.
    ///
    /// Corresponds to SQLSTATE code 40001
    ///
    /// This error is only detected for PostgreSQL, as we do not yet support
    /// transaction isolation levels for other backends.
    SerializationFailure,

    /// The command could not be completed because the transaction was read
    /// only.
    ///
    /// This error will also be returned for `SELECT` statements which attempted
    /// to lock the rows.
    ReadOnlyTransaction,

    /// A not null constraint was violated.
    NotNullViolation,

    /// A check constraint was violated.
    CheckViolation,

    #[doc(hidden)]
    Unknown, // Match against _ instead, more variants may be added in the future
}

/// Information about an error that was returned by the database.
pub trait DatabaseErrorInformation {
    /// The primary human-readable error message. Typically one line.
    fn message(&self) -> &str;

    /// An optional secondary error message providing more details about the
    /// problem, if it was provided by the database. Might span multiple lines.
    fn details(&self) -> Option<&str>;

    /// An optional suggestion of what to do about the problem, if one was
    /// provided by the database.
    fn hint(&self) -> Option<&str>;

    /// The name of the table the error was associated with, if the error was
    /// associated with a specific table and the backend supports retrieving
    /// that information.
    ///
    /// Currently this method will return `None` for all backends other than
    /// PostgreSQL.
    fn table_name(&self) -> Option<&str>;

    /// The name of the column the error was associated with, if the error was
    /// associated with a specific column and the backend supports retrieving
    /// that information.
    ///
    /// Currently this method will return `None` for all backends other than
    /// PostgreSQL.
    fn column_name(&self) -> Option<&str>;

    /// The constraint that was violated if this error is a constraint violation
    /// and the backend supports retrieving that information.
    ///
    /// Currently this method will return `None` for all backends other than
    /// PostgreSQL.
    fn constraint_name(&self) -> Option<&str>;

    /// An optional integer indicating an error cursor position as an index into
    /// the original statement string.
    fn statement_position(&self) -> Option<i32>;
}

impl fmt::Debug for dyn DatabaseErrorInformation + Send + Sync {
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
    fn statement_position(&self) -> Option<i32> {
        None
    }
}

/// Errors which can occur during [`Connection::establish`]
///
/// [`Connection::establish`]: crate::connection::Connection::establish
#[derive(Debug, PartialEq)]
#[non_exhaustive]
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

/// See the [method documentation](OptionalExtension::optional).
pub trait OptionalExtension<T> {
    /// Converts a `QueryResult<T>` into a `QueryResult<Option<T>>`.
    ///
    /// By default, Diesel treats 0 rows being returned from a query that is expected to return 1
    /// row as an error (e.g. the return value of [`get_result`] or [`first`]). This method will
    /// handle that error, and give you back an `Option<T>` instead.
    ///
    /// [`get_result`]: crate::query_dsl::RunQueryDsl::get_result()
    /// [`first`]: crate::query_dsl::RunQueryDsl::first()
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::{QueryResult, NotFound, OptionalExtension};
    ///
    /// let result: QueryResult<i32> = Ok(1);
    /// assert_eq!(Ok(Some(1)), result.optional());
    ///
    /// let result: QueryResult<i32> = Err(NotFound);
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
            Error::InvalidCString(ref nul_err) => write!(f, "{}", nul_err),
            Error::DatabaseError(_, ref e) => write!(f, "{}", e.message()),
            Error::NotFound => f.write_str("Record not found"),
            Error::QueryBuilderError(ref e) => e.fmt(f),
            Error::DeserializationError(ref e) => e.fmt(f),
            Error::SerializationError(ref e) => e.fmt(f),
            Error::RollbackError(ref e) => e.fmt(f),
            Error::RollbackTransaction => write!(f, "The current transaction was aborted"),
            Error::AlreadyInTransaction => write!(
                f,
                "Cannot perform this operation while a transaction is open",
            ),
        }
    }
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            Error::InvalidCString(ref e) => Some(e),
            Error::QueryBuilderError(ref e) => Some(&**e),
            Error::DeserializationError(ref e) => Some(&**e),
            Error::SerializationError(ref e) => Some(&**e),
            _ => None,
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
        }
    }
}

impl StdError for ConnectionError {
    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            ConnectionError::InvalidCString(ref e) => Some(e),
            ConnectionError::CouldntSetupConfiguration(ref e) => Some(e),
            _ => None,
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
            (&Error::RollbackTransaction, &Error::RollbackTransaction) => true,
            (&Error::AlreadyInTransaction, &Error::AlreadyInTransaction) => true,
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

/// An unexpected `NULL` was encountered during deserialization
#[derive(Debug, Clone, Copy)]
pub struct UnexpectedNullError;

impl fmt::Display for UnexpectedNullError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unexpected null for non-null column")
    }
}

impl StdError for UnexpectedNullError {}

/// Expected more fields then present in the current row while deserialising results
#[derive(Debug, Clone, Copy)]
pub struct UnexpectedEndOfRow;

impl fmt::Display for UnexpectedEndOfRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unexpected end of row")
    }
}

impl StdError for UnexpectedEndOfRow {}
