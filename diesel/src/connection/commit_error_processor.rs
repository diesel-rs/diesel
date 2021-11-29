//! A module to evaluate what to do when a commit triggers an error.
use crate::result::{DatabaseErrorKind, Error};

/// Transaction status returned upon error on commit.
#[derive(Debug)]
pub enum CommitErrorOutcome {
    /// Necessitates a rollback to return to a valid transaction
    RollbackAndThrow(Error),
    /// Broken transaction. Returned if an error has occurred earlier in a Postgres transaction.
    Throw(Error),
    /// Broken transaction. Similar to `Throw`, but marks the manager as broken. It should switch
    /// to `TransactionManagerStatus::InError` and refuse to run additional operations.
    ThrowAndMarkManagerAsBroken(Error),
}

/// Trait needed for the transaction manager.
pub trait CommitErrorProcessor {
    /// Returns the status of the transaction following an error upon commit.
    /// When any of these kinds of error happen on `COMMIT`, it is expected
    /// that a `ROLLBACK` would succeed, leaving the transaction in a non-broken state.
    /// If there are other such errors, it is fine to add them here.
    fn process_commit_error(&self, transaction_depth: i32, error: Error) -> CommitErrorOutcome;
}

/// Default implementation of CommitErrorProcessor::process_commit_error(), used for MySql and
/// Sqlite connections. Returns `CommitErrorOutcome::RollbackAndThrow` if the transaction depth is
/// greater than 1, the error is a `DatabaseError` and the error kind is either
/// `DatabaseErrorKind::SerializationFailure` or `DatabaseErrorKind::ReadOnlyTransaction`
pub fn default_process_commit_error(transaction_depth: i32, error: Error) -> CommitErrorOutcome {
    match error {
        Error::DatabaseError(DatabaseErrorKind::ReadOnlyTransaction, _)
        | Error::DatabaseError(DatabaseErrorKind::SerializationFailure, _)
            if transaction_depth <= 1 =>
        {
            CommitErrorOutcome::RollbackAndThrow(error)
        }
        Error::AlreadyInTransaction
        | Error::DatabaseError(DatabaseErrorKind::CheckViolation, _)
        | Error::DatabaseError(DatabaseErrorKind::ClosedConnection, _)
        | Error::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _)
        | Error::DatabaseError(DatabaseErrorKind::NotNullViolation, _)
        | Error::DatabaseError(DatabaseErrorKind::ReadOnlyTransaction, _)
        | Error::DatabaseError(DatabaseErrorKind::SerializationFailure, _)
        | Error::DatabaseError(DatabaseErrorKind::UnableToSendCommand, _)
        | Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)
        | Error::DatabaseError(DatabaseErrorKind::Unknown, _)
        | Error::DeserializationError(_)
        | Error::InvalidCString(_)
        | Error::NotFound
        | Error::QueryBuilderError(_)
        | Error::RollbackError(_)
        | Error::RollbackTransaction
        | Error::SerializationError(_)
        | Error::NotInTransaction
        | Error::BrokenTransaction => CommitErrorOutcome::Throw(error),
    }
}
