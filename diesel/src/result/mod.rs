//! Error types and `Result` wrappers

/// Generic errors
mod error;
pub use self::error::{Error, DatabaseErrorKind, ConnectionError};

/// Database error information
mod database_error_information;
pub use self::database_error_information::DatabaseErrorInformation;

/// Transaction specific errors
mod transaction_error;
pub use self::transaction_error::TransactionError;

/// Trait to handle errors that actually mean "optional value" (for `NotFound`)
mod optional;
pub use self::optional::OptionalExtension;


/// A result with a `diesel::result::Error`
pub type QueryResult<T> = Result<T, Error>;

/// A result with a `diesel::result::ConnectionError`
pub type ConnectionResult<T> = Result<T, ConnectionError>;

/// A result with a `diesel::result::TransactionErrorError`
pub type TransactionResult<T, E> = Result<T, TransactionError<E>>;


#[cfg(test)]
#[allow(warnings)]
fn error_impls_send() {
    let err: Error = unimplemented!();
    let x: &Send = &err;
}
