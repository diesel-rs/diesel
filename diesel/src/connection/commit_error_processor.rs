//! A module to evaluate what to do when a commit triggers an error.
use crate::result::{DatabaseErrorKind, Error};

/// Transaction status returned upon error on commit.
#[derive(Debug)]
pub enum CommitErrorOutcome {
    /// Necessitates a rollback to return to a valid transaction
    RollbackAndThrow(Error),
    /// Broken transaction. Returned if an error has occurred earlier in a Postgres transaction.
    Throw(Error),
}

/// Trait needed for the transaction manager.
pub trait CommitErrorProcessor {
    /// Returns the status of the transaction following an error upon commit.
    // When any of these kinds of error happen on `COMMIT`, it is expected
    // that a `ROLLBACK` would succeed, leaving the transaction in a non-broken state.
    // If there are other such errors, it is fine to add them here.
    fn process_commit_error(&self, transaction_depth: i32, error: Error) -> CommitErrorOutcome {
        match error {
            Error::DatabaseError(DatabaseErrorKind::SerializationFailure, _)
            | Error::DatabaseError(DatabaseErrorKind::ReadOnlyTransaction, _)
                if transaction_depth <= 1 =>
            {
                CommitErrorOutcome::RollbackAndThrow(error)
            }
            _ => CommitErrorOutcome::Throw(error),
        }
    }
}
