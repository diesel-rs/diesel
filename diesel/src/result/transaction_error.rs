use ::std::fmt::{self, Display};
use ::std::error::Error as StdError;

use super::Error;

// Can't use the quick-error macro because of missing generics support,
// see <https://github.com/tailhook/quick-error/issues/20> for more information.

#[derive(Debug, PartialEq)]
pub enum TransactionError<E> {
    CouldntCreateTransaction(Error),
    UserReturnedError(E),
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
