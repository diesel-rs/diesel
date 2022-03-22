//! A module to evaluate what to do when a commit triggers an error.
use crate::result::Error;

/// Transaction status returned upon error on commit.
#[derive(Debug)]
#[non_exhaustive]
#[cfg_attr(
    doc_cfg,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
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
#[cfg_attr(
    doc_cfg,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
pub trait CommitErrorProcessor {
    /// Returns the status of the transaction following an error upon commit.
    /// When any of these kinds of error happen on `COMMIT`, it is expected
    /// that a `ROLLBACK` would succeed, leaving the transaction in a non-broken state.
    /// If there are other such errors, it is fine to add them here.
    fn process_commit_error(&self, error: Error) -> CommitErrorOutcome;
}

/// Default implementation of CommitErrorProcessor::process_commit_error(), used for MySql and
/// Sqlite connections. Returns `CommitErrorOutcome::RollbackAndThrow` if the transaction depth is
/// greater than 1, the error is a `DatabaseError` and the error kind is either
/// `DatabaseErrorKind::SerializationFailure` or `DatabaseErrorKind::ReadOnlyTransaction`
#[cfg_attr(
    doc_cfg,
    doc(cfg(feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"))
)]
#[cfg(any(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    feature = "mysql",
    feature = "sqlite"
))]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
pub(crate) fn default_process_commit_error(
    transaction_state: &super::ValidTransactionManagerStatus,
    error: Error,
) -> CommitErrorOutcome {
    use crate::result::DatabaseErrorKind;

    if let Some(transaction_depth) = transaction_state.transaction_depth() {
        match error {
            // Neither mysql nor sqlite do currently produce these errors
            // we keep this match arm here for the case we may generate
            // such errors in future versions of diesel
            Error::DatabaseError(DatabaseErrorKind::ReadOnlyTransaction, _)
            | Error::DatabaseError(DatabaseErrorKind::SerializationFailure, _)
                if transaction_depth.get() == 1 =>
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
            | Error::BrokenTransaction
            | Error::CommitTransactionFailed { .. } => CommitErrorOutcome::Throw(error),
        }
    } else {
        unreachable!(
            "Calling commit_error_processor outside of a transaction is implementation error.\
            If you ever see this error message outside implementing a custom transaction manager\
            please open a new issue at diesels issue tracker."
        )
    }
}

#[cfg(test)]
#[cfg(any(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    feature = "mysql",
    feature = "sqlite"
))]
mod tests {
    use super::CommitErrorOutcome;
    use crate::connection::ValidTransactionManagerStatus;
    use crate::result::{DatabaseErrorKind, Error};
    use std::num::NonZeroU32;

    #[test]
    fn check_default_process_commit_error_implementation() {
        let state = ValidTransactionManagerStatus {
            // Transaction depth == 1, so one unnested transaction
            transaction_depth: NonZeroU32::new(1),
            previous_serialization_error: None,
        };
        let error = Error::DatabaseError(
            DatabaseErrorKind::ReadOnlyTransaction,
            Box::new(String::from("whatever")),
        );
        let action = super::default_process_commit_error(&state, error);
        assert!(matches!(action, CommitErrorOutcome::RollbackAndThrow(_)));

        let error = Error::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new(String::from("whatever")),
        );
        let action = super::default_process_commit_error(&state, error);
        assert!(matches!(action, CommitErrorOutcome::Throw(_)));

        let state = ValidTransactionManagerStatus {
            // Transaction depth == 2, so two nested transactions
            transaction_depth: NonZeroU32::new(2),
            previous_serialization_error: None,
        };
        let error = Error::DatabaseError(
            DatabaseErrorKind::ReadOnlyTransaction,
            Box::new(String::from("whatever")),
        );
        let action = super::default_process_commit_error(&state, error);
        assert!(matches!(action, CommitErrorOutcome::Throw(_)));
    }

    #[test]
    #[should_panic]
    fn check_invalid_transaction_state_rejected() {
        let state = ValidTransactionManagerStatus {
            // Transaction depth == None, so no transaction running, so nothing
            // to rollback. Something went wrong so mark everything as broken.
            transaction_depth: None,
            previous_serialization_error: None,
        };
        let error = Error::DatabaseError(
            DatabaseErrorKind::ReadOnlyTransaction,
            Box::new(String::from("whatever")),
        );
        let action = super::default_process_commit_error(&state, error);
        assert!(matches!(
            action,
            CommitErrorOutcome::ThrowAndMarkManagerAsBroken(_)
        ));
    }
}
