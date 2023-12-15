use crate::connection::Connection;
use crate::result::{Error, QueryResult};
use std::borrow::Cow;
use std::num::NonZeroU32;

/// Manages the internal transaction state for a connection.
///
/// You will not need to interact with this trait, unless you are writing an
/// implementation of [`Connection`].
pub trait TransactionManager<Conn: Connection> {
    /// Data stored as part of the connection implementation
    /// to track the current transaction state of a connection
    type TransactionStateData;

    /// Begin a new transaction or savepoint
    ///
    /// If the transaction depth is greater than 0,
    /// this should create a savepoint instead.
    /// This function is expected to increment the transaction depth by 1.
    fn begin_transaction(conn: &mut Conn) -> QueryResult<()>;

    /// Rollback the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should rollback to the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn rollback_transaction(conn: &mut Conn) -> QueryResult<()>;

    /// Commit the inner-most transaction or savepoint
    ///
    /// If the transaction depth is greater than 1,
    /// this should release the most recent savepoint.
    /// This function is expected to decrement the transaction depth by 1.
    fn commit_transaction(conn: &mut Conn) -> QueryResult<()>;

    /// Fetch the current transaction status as mutable
    ///
    /// Used to ensure that `begin_test_transaction` is not called when already
    /// inside of a transaction, and that operations are not run in a `InError`
    /// transaction manager.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus;

    /// Executes the given function inside of a database transaction
    ///
    /// Each implementation of this function needs to fulfill the documented
    /// behaviour of [`Connection::transaction`]
    fn transaction<F, R, E>(conn: &mut Conn, callback: F) -> Result<R, E>
    where
        F: FnOnce(&mut Conn) -> Result<R, E>,
        E: From<Error>,
    {
        Self::begin_transaction(conn)?;
        match callback(&mut *conn) {
            Ok(value) => {
                Self::commit_transaction(conn)?;
                Ok(value)
            }
            Err(user_error) => match Self::rollback_transaction(conn) {
                Ok(()) => Err(user_error),
                Err(Error::BrokenTransactionManager) => {
                    // In this case we are probably more interested by the
                    // original error, which likely caused this
                    Err(user_error)
                }
                Err(rollback_error) => Err(rollback_error.into()),
            },
        }
    }

    /// This methods checks if the connection manager is considered to be broken
    /// by connection pool implementations
    ///
    /// A connection manager is considered to be broken by default if it either
    /// contains an open transaction (because you don't want to have connections
    /// with open transactions in your pool) or when the transaction manager is
    /// in an error state.
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    fn is_broken_transaction_manager(conn: &mut Conn) -> bool {
        match Self::transaction_manager_status_mut(conn).transaction_state() {
            // all transactions are closed
            // so we don't consider this connection broken
            Ok(ValidTransactionManagerStatus {
                in_transaction: None,
            }) => false,
            // The transaction manager is in an error state
            // Therefore we consider this connection broken
            Err(_) => true,
            // The transaction manager contains a open transaction
            // we do consider this connection broken
            // if that transaction was not opened by `begin_test_transaction`
            Ok(ValidTransactionManagerStatus {
                in_transaction: Some(s),
            }) => !s.test_transaction,
        }
    }
}

/// An implementation of `TransactionManager` which can be used for backends
/// which use ANSI standard syntax for savepoints such as SQLite and PostgreSQL.
#[derive(Default, Debug)]
pub struct AnsiTransactionManager {
    pub(crate) status: TransactionManagerStatus,
}

/// Status of the transaction manager
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
)]
#[derive(Debug)]
pub enum TransactionManagerStatus {
    /// Valid status, the manager can run operations
    Valid(ValidTransactionManagerStatus),
    /// Error status, probably following a broken connection. The manager will no longer run operations
    InError,
}

impl Default for TransactionManagerStatus {
    fn default() -> Self {
        TransactionManagerStatus::Valid(ValidTransactionManagerStatus::default())
    }
}

impl TransactionManagerStatus {
    /// Returns the transaction depth if the transaction manager's status is valid, or returns
    /// [`Error::BrokenTransactionManager`] if the transaction manager is in error.
    pub fn transaction_depth(&self) -> QueryResult<Option<NonZeroU32>> {
        match self {
            TransactionManagerStatus::Valid(valid_status) => Ok(valid_status.transaction_depth()),
            TransactionManagerStatus::InError => Err(Error::BrokenTransactionManager),
        }
    }

    #[cfg(any(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
        feature = "postgres",
        feature = "mysql",
        test
    ))]
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    /// If in transaction and transaction manager is not broken, registers that it's possible that
    /// the connection can not be used anymore until top-level transaction is rolled back.
    ///
    /// If that is registered, savepoints rollbacks will still be attempted, but failure to do so
    /// will not result in an error. (Some may succeed, some may not.)
    pub(crate) fn set_requires_rollback_maybe_up_to_top_level(&mut self, to: bool) {
        if let TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
            in_transaction:
                Some(InTransactionStatus {
                    requires_rollback_maybe_up_to_top_level,
                    ..
                }),
        }) = self
        {
            *requires_rollback_maybe_up_to_top_level = to;
        }
    }

    /// Sets the transaction manager status to InError
    ///
    /// Subsequent attempts to use transaction-related features will result in a
    /// [`Error::BrokenTransactionManager`] error
    pub fn set_in_error(&mut self) {
        *self = TransactionManagerStatus::InError
    }

    /// Expose access to the inner transaction state
    ///
    /// This function returns an error if the Transaction manager is in a broken
    /// state
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(self) fn transaction_state(&mut self) -> QueryResult<&mut ValidTransactionManagerStatus> {
        match self {
            TransactionManagerStatus::Valid(valid_status) => Ok(valid_status),
            TransactionManagerStatus::InError => Err(Error::BrokenTransactionManager),
        }
    }

    /// This function allows to flag a transaction manager
    /// in such a way that it contains a test transaction.
    ///
    /// This will disable some checks in regards to open transactions
    /// to allow `Connection::begin_test_transaction` to work with
    /// pooled connections as well
    #[diesel_derives::__diesel_public_if(
        feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes"
    )]
    pub(crate) fn set_test_transaction_flag(&mut self) {
        if let TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
            in_transaction: Some(s),
        }) = self
        {
            s.test_transaction = true;
        }
    }
}

/// Valid transaction status for the manager. Can return the current transaction depth
#[allow(missing_copy_implementations)]
#[derive(Debug, Default)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(in_transaction)
)]
pub struct ValidTransactionManagerStatus {
    /// Inner status, or `None` if no transaction is running
    in_transaction: Option<InTransactionStatus>,
}

/// Various status fields to track the status of
/// a transaction manager with a started transaction
#[allow(missing_copy_implementations)]
#[derive(Debug)]
#[diesel_derives::__diesel_public_if(
    feature = "i-implement-a-third-party-backend-and-opt-into-breaking-changes",
    public_fields(
        test_transaction,
        transaction_depth,
        requires_rollback_maybe_up_to_top_level
    )
)]
pub struct InTransactionStatus {
    /// The current depth of nested transactions
    transaction_depth: NonZeroU32,
    /// If that is registered, savepoints rollbacks will still be attempted, but failure to do so
    /// will not result in an error. (Some may succeed, some may not.)
    requires_rollback_maybe_up_to_top_level: bool,
    /// Is this transaction manager status marked as test-transaction?
    test_transaction: bool,
}

impl ValidTransactionManagerStatus {
    /// Return the current transaction depth
    ///
    /// This value is `None` if no current transaction is running
    /// otherwise the number of nested transactions is returned.
    pub fn transaction_depth(&self) -> Option<NonZeroU32> {
        self.in_transaction.as_ref().map(|it| it.transaction_depth)
    }

    /// Update the transaction depth by adding the value of the `transaction_depth_change` parameter if the `query` is
    /// `Ok(())`
    pub fn change_transaction_depth(
        &mut self,
        transaction_depth_change: TransactionDepthChange,
    ) -> QueryResult<()> {
        match (&mut self.in_transaction, transaction_depth_change) {
            (Some(in_transaction), TransactionDepthChange::IncreaseDepth) => {
                // Can be replaced with saturating_add directly on NonZeroU32 once
                // <https://github.com/rust-lang/rust/issues/84186> is stable
                in_transaction.transaction_depth =
                    NonZeroU32::new(in_transaction.transaction_depth.get().saturating_add(1))
                        .expect("nz + nz is always non-zero");
                Ok(())
            }
            (Some(in_transaction), TransactionDepthChange::DecreaseDepth) => {
                // This sets `transaction_depth` to `None` as soon as we reach zero
                match NonZeroU32::new(in_transaction.transaction_depth.get() - 1) {
                    Some(depth) => in_transaction.transaction_depth = depth,
                    None => self.in_transaction = None,
                }
                Ok(())
            }
            (None, TransactionDepthChange::IncreaseDepth) => {
                self.in_transaction = Some(InTransactionStatus {
                    transaction_depth: NonZeroU32::new(1).expect("1 is non-zero"),
                    requires_rollback_maybe_up_to_top_level: false,
                    test_transaction: false,
                });
                Ok(())
            }
            (None, TransactionDepthChange::DecreaseDepth) => {
                // We screwed up something somewhere
                // we cannot decrease the transaction count if
                // we are not inside a transaction
                Err(Error::NotInTransaction)
            }
        }
    }
}

/// Represents a change to apply to the depth of a transaction
#[derive(Debug, Clone, Copy)]
pub enum TransactionDepthChange {
    /// Increase the depth of the transaction (corresponds to `BEGIN` or `SAVEPOINT`)
    IncreaseDepth,
    /// Decreases the depth of the transaction (corresponds to `COMMIT`/`RELEASE SAVEPOINT` or `ROLLBACK`)
    DecreaseDepth,
}

impl AnsiTransactionManager {
    fn get_transaction_state<Conn>(
        conn: &mut Conn,
    ) -> QueryResult<&mut ValidTransactionManagerStatus>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        conn.transaction_state().status.transaction_state()
    }

    /// Begin a transaction with custom SQL
    ///
    /// This is used by connections to implement more complex transaction APIs
    /// to set things such as isolation levels.
    /// Returns an error if already inside of a transaction.
    pub fn begin_transaction_sql<Conn>(conn: &mut Conn, sql: &str) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self>,
    {
        let state = Self::get_transaction_state(conn)?;
        match state.transaction_depth() {
            None => {
                conn.batch_execute(sql)?;
                Self::get_transaction_state(conn)?
                    .change_transaction_depth(TransactionDepthChange::IncreaseDepth)?;
                Ok(())
            }
            Some(_depth) => Err(Error::AlreadyInTransaction),
        }
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager
where
    Conn: Connection<TransactionManager = Self>,
{
    type TransactionStateData = Self;

    fn begin_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let transaction_depth = transaction_state.transaction_depth();
        let start_transaction_sql = match transaction_depth {
            None => Cow::from("BEGIN"),
            Some(transaction_depth) => {
                Cow::from(format!("SAVEPOINT diesel_savepoint_{transaction_depth}"))
            }
        };
        conn.instrumentation().on_connection_event(
            super::instrumentation::InstrumentationEvent::BeginTransaction {
                depth: NonZeroU32::new(
                    transaction_depth.map_or(0, NonZeroU32::get).wrapping_add(1),
                )
                .expect("Transaction depth is too large"),
            },
        );
        conn.batch_execute(&start_transaction_sql)?;
        Self::get_transaction_state(conn)?
            .change_transaction_depth(TransactionDepthChange::IncreaseDepth)?;

        Ok(())
    }

    fn rollback_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;

        let (
            (rollback_sql, rolling_back_top_level),
            requires_rollback_maybe_up_to_top_level_before_execute,
        ) = match transaction_state.in_transaction {
            Some(ref in_transaction) => (
                match in_transaction.transaction_depth.get() {
                    1 => (Cow::Borrowed("ROLLBACK"), true),
                    depth_gt1 => (
                        Cow::Owned(format!(
                            "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                            depth_gt1 - 1
                        )),
                        false,
                    ),
                },
                in_transaction.requires_rollback_maybe_up_to_top_level,
            ),
            None => return Err(Error::NotInTransaction),
        };
        let depth = transaction_state
            .transaction_depth()
            .expect("We know that we are in a transaction here");
        conn.instrumentation().on_connection_event(
            super::instrumentation::InstrumentationEvent::RollbackTransaction { depth },
        );

        match conn.batch_execute(&rollback_sql) {
            Ok(()) => {
                match Self::get_transaction_state(conn)?
                    .change_transaction_depth(TransactionDepthChange::DecreaseDepth)
                {
                    Ok(()) => {}
                    Err(Error::NotInTransaction) if rolling_back_top_level => {
                        // Transaction exit may have already been detected by connection
                        // implementation. It's fine.
                    }
                    Err(e) => return Err(e),
                }
                Ok(())
            }
            Err(rollback_error) => {
                let tm_status = Self::transaction_manager_status_mut(conn);
                match tm_status {
                    TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                        in_transaction:
                            Some(InTransactionStatus {
                                transaction_depth,
                                requires_rollback_maybe_up_to_top_level,
                                ..
                            }),
                    }) if transaction_depth.get() > 1 => {
                        // A savepoint failed to rollback - we may still attempt to repair
                        // the connection by rolling back higher levels.

                        // To make it easier on the user (that they don't have to really
                        // look at actual transaction depth and can just rely on the number
                        // of times they have called begin/commit/rollback) we still
                        // decrement here:
                        *transaction_depth = NonZeroU32::new(transaction_depth.get() - 1)
                            .expect("Depth was checked to be > 1");
                        *requires_rollback_maybe_up_to_top_level = true;
                        if requires_rollback_maybe_up_to_top_level_before_execute {
                            // In that case, we tolerate that savepoint releases fail
                            // -> we should ignore errors
                            return Ok(());
                        }
                    }
                    TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                        in_transaction: None,
                    }) => {
                        // we would have returned `NotInTransaction` if that was already the state
                        // before we made our call
                        // => Transaction manager status has been fixed by the underlying connection
                        // so we don't need to set_in_error
                    }
                    _ => tm_status.set_in_error(),
                }
                Err(rollback_error)
            }
        }
    }

    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be returned. In the second case the connection will be considered broken
    /// as it contains a uncommitted unabortable open transaction.
    fn commit_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let transaction_depth = transaction_state.transaction_depth();
        let (commit_sql, committing_top_level) = match transaction_depth {
            None => return Err(Error::NotInTransaction),
            Some(transaction_depth) if transaction_depth.get() == 1 => {
                (Cow::Borrowed("COMMIT"), true)
            }
            Some(transaction_depth) => (
                Cow::Owned(format!(
                    "RELEASE SAVEPOINT diesel_savepoint_{}",
                    transaction_depth.get() - 1
                )),
                false,
            ),
        };
        let depth = transaction_state
            .transaction_depth()
            .expect("We know that we are in a transaction here");
        conn.instrumentation().on_connection_event(
            super::instrumentation::InstrumentationEvent::CommitTransaction { depth },
        );
        match conn.batch_execute(&commit_sql) {
            Ok(()) => {
                match Self::get_transaction_state(conn)?
                    .change_transaction_depth(TransactionDepthChange::DecreaseDepth)
                {
                    Ok(()) => {}
                    Err(Error::NotInTransaction) if committing_top_level => {
                        // Transaction exit may have already been detected by connection.
                        // It's fine
                    }
                    Err(e) => return Err(e),
                }
                Ok(())
            }
            Err(commit_error) => {
                if let TransactionManagerStatus::Valid(ValidTransactionManagerStatus {
                    in_transaction:
                        Some(InTransactionStatus {
                            requires_rollback_maybe_up_to_top_level: true,
                            ..
                        }),
                }) = conn.transaction_state().status
                {
                    match Self::rollback_transaction(conn) {
                        Ok(()) => {}
                        Err(rollback_error) => {
                            conn.transaction_state().status.set_in_error();
                            return Err(Error::RollbackErrorOnCommit {
                                rollback_error: Box::new(rollback_error),
                                commit_error: Box::new(commit_error),
                            });
                        }
                    }
                }
                Err(commit_error)
            }
        }
    }

    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus {
        &mut conn.transaction_state().status
    }
}

#[cfg(test)]
// that's a false positive for `panic!`/`assert!` on rust 2018
#[allow(clippy::uninlined_format_args)]
mod test {
    // Mock connection.
    mod mock {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::Instrumentation;
        use crate::connection::{
            Connection, ConnectionSealed, SimpleConnection, TransactionManager,
        };
        use crate::result::QueryResult;
        use crate::test_helpers::TestConnection;
        use std::collections::VecDeque;

        pub(crate) struct MockConnection {
            pub(crate) next_results: VecDeque<QueryResult<usize>>,
            pub(crate) next_batch_execute_results: VecDeque<QueryResult<()>>,
            pub(crate) top_level_requires_rollback_after_next_batch_execute: bool,
            transaction_state: AnsiTransactionManager,
            instrumentation: Option<Box<dyn Instrumentation>>,
        }

        impl SimpleConnection for MockConnection {
            fn batch_execute(&mut self, _query: &str) -> QueryResult<()> {
                let res = self
                    .next_batch_execute_results
                    .pop_front()
                    .expect("No next result");
                if self.top_level_requires_rollback_after_next_batch_execute {
                    self.transaction_state
                        .status
                        .set_requires_rollback_maybe_up_to_top_level(true);
                }
                res
            }
        }

        impl ConnectionSealed for MockConnection {}

        impl Connection for MockConnection {
            type Backend = <TestConnection as Connection>::Backend;

            type TransactionManager = AnsiTransactionManager;

            fn establish(_database_url: &str) -> crate::ConnectionResult<Self> {
                Ok(Self {
                    next_results: VecDeque::new(),
                    next_batch_execute_results: VecDeque::new(),
                    top_level_requires_rollback_after_next_batch_execute: false,
                    transaction_state: AnsiTransactionManager::default(),
                    instrumentation: None,
                })
            }

            fn execute_returning_count<T>(&mut self, _source: &T) -> QueryResult<usize>
            where
                T: crate::query_builder::QueryFragment<Self::Backend>
                    + crate::query_builder::QueryId,
            {
                self.next_results.pop_front().expect("No next result")
            }

            fn transaction_state(
                &mut self,
            ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData
            {
                &mut self.transaction_state
            }

            fn instrumentation(&mut self) -> &mut dyn crate::connection::Instrumentation {
                &mut self.instrumentation
            }

            fn set_instrumentation(
                &mut self,
                instrumentation: impl crate::connection::Instrumentation,
            ) {
                self.instrumentation = Some(Box::new(instrumentation));
            }
        }
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn transaction_manager_returns_an_error_when_attempting_to_commit_outside_of_a_transaction() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::result::Error;
        use crate::PgConnection;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = AnsiTransactionManager::commit_transaction(conn);
        assert!(matches!(result, Err(Error::NotInTransaction)))
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn transaction_manager_returns_an_error_when_attempting_to_rollback_outside_of_a_transaction() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::result::Error;
        use crate::PgConnection;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = AnsiTransactionManager::rollback_transaction(conn);
        assert!(matches!(result, Err(Error::NotInTransaction)))
    }

    #[test]
    fn transaction_manager_enters_broken_state_when_connection_is_broken() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::connection::TransactionManagerStatus;
        use crate::result::{DatabaseErrorKind, Error};
        use crate::*;

        let mut conn = mock::MockConnection::establish("mock").expect("Mock connection");

        // Set result for BEGIN
        conn.next_batch_execute_results.push_back(Ok(()));
        let result = conn.transaction(|conn| {
            conn.next_results.push_back(Ok(1));
            let query_result = sql_query("SELECT 1").execute(conn);
            assert!(query_result.is_ok());
            // Set result for COMMIT attempt
            conn.next_batch_execute_results
                .push_back(Err(Error::DatabaseError(
                    DatabaseErrorKind::Unknown,
                    Box::new("commit fails".to_string()),
                )));
            conn.top_level_requires_rollback_after_next_batch_execute = true;
            conn.next_batch_execute_results
                .push_back(Err(Error::DatabaseError(
                    DatabaseErrorKind::Unknown,
                    Box::new("rollback also fails".to_string()),
                )));
            Ok(())
        });
        assert!(
            matches!(
                &result,
                Err(Error::RollbackErrorOnCommit {
                    rollback_error,
                    commit_error
                }) if matches!(**commit_error, Error::DatabaseError(DatabaseErrorKind::Unknown, _))
                    && matches!(&**rollback_error,
                        Error::DatabaseError(DatabaseErrorKind::Unknown, msg)
                            if msg.message() == "rollback also fails"
                    )
            ),
            "Got {:?}",
            result
        );
        assert!(matches!(
            *AnsiTransactionManager::transaction_manager_status_mut(&mut conn),
            TransactionManagerStatus::InError
        ));
        // Ensure the transaction manager is unusable
        let result = conn.transaction(|_conn| Ok(()));
        assert!(matches!(result, Err(Error::BrokenTransactionManager)))
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn mysql_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::*;
        use std::num::NonZeroU32;

        let conn = &mut crate::test_helpers::connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.transaction(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            // In MySQL, a syntax error does not break the transaction block
            let query_result = sql_query("SELECT_SYNTAX_ERROR 1").execute(conn);
            assert!(query_result.is_err());
            query_result
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn sqlite_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::*;
        use std::num::NonZeroU32;

        let conn = &mut crate::test_helpers::connection();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.transaction(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            // In Sqlite, a syntax error does not break the transaction block
            let query_result = sql_query("SELECT_SYNTAX_ERROR 1").execute(conn);
            assert!(query_result.is_err());
            query_result
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn nested_mysql_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::*;
        use std::num::NonZeroU32;

        let conn = &mut crate::test_helpers::connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = conn.transaction(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let result = conn.transaction(|conn| {
                assert_eq!(
                    NonZeroU32::new(2),
                    <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                        conn
            ).transaction_depth().expect("Transaction depth")
                );
                // In MySQL, a syntax error does not break the transaction block
                sql_query("SELECT_SYNTAX_ERROR 1").execute(conn)
            });
            assert!(result.is_err());
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let query_result = sql_query("SELECT 1").execute(conn);
            assert!(query_result.is_ok());
            query_result
        });
        assert!(result.is_ok());
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    // This function uses a collect with side effects (spawning threads)
    // so clippy is wrong here
    #[allow(clippy::needless_collect)]
    fn mysql_transaction_depth_commits_tracked_properly_on_serialization_failure() {
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::DatabaseError;
        use crate::*;
        use std::num::NonZeroU32;
        use std::sync::{Arc, Barrier};
        use std::thread;

        table! {
            #[sql_name = "mysql_transaction_depth_is_tracked_properly_on_commit_failure"]
            serialization_example {
                id -> Integer,
                class -> Integer,
            }
        }

        let conn = &mut crate::test_helpers::connection_no_transaction();

        sql_query(
            "DROP TABLE IF EXISTS mysql_transaction_depth_is_tracked_properly_on_commit_failure;",
        )
        .execute(conn)
        .unwrap();
        sql_query(
            r#"
            CREATE TABLE mysql_transaction_depth_is_tracked_properly_on_commit_failure (
                id INT AUTO_INCREMENT PRIMARY KEY,
                class INTEGER NOT NULL
            )
        "#,
        )
        .execute(conn)
        .unwrap();

        insert_into(serialization_example::table)
            .values(&vec![
                serialization_example::class.eq(1),
                serialization_example::class.eq(2),
            ])
            .execute(conn)
            .unwrap();

        let before_barrier = Arc::new(Barrier::new(2));
        let after_barrier = Arc::new(Barrier::new(2));

        let threads = (1..3)
            .map(|i| {
                let before_barrier = before_barrier.clone();
                let after_barrier = after_barrier.clone();
                thread::spawn(move || {
                    use crate::connection::transaction_manager::AnsiTransactionManager;
                    use crate::connection::transaction_manager::TransactionManager;
                    let conn = &mut crate::test_helpers::connection_no_transaction();
                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                    crate::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                    let result =
                    conn.transaction(|conn| {
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                        let _ = serialization_example::table
                            .filter(serialization_example::class.eq(i))
                            .count()
                            .execute(conn)?;

                        let other_i = if i == 1 { 2 } else { 1 };
                        let q = insert_into(serialization_example::table)
                            .values(serialization_example::class.eq(other_i));
                        before_barrier.wait();

                        let r = q.execute(conn);
                        after_barrier.wait();
                        r
                    });

                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let second_trans_result = conn.transaction(|conn| crate::sql_query("SELECT 1").execute(conn));
                    assert!(second_trans_result.is_ok(), "Expected the thread connections to have been rolled back or committed, but second transaction exited with {:?}", second_trans_result);
                    result
                })
            })
            .collect::<Vec<_>>();
        let second_trans_result =
            conn.transaction(|conn| crate::sql_query("SELECT 1").execute(conn));
        assert!(second_trans_result.is_ok(), "Expected the main connection to have been rolled back or committed, but second transaction exited with {:?}", second_trans_result);

        let mut results = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();

        results.sort_by_key(|r| r.is_err());
        assert!(results[0].is_ok(), "Got {:?} instead", results);
        // Note that contrary to Postgres, this is not a commit failure
        assert!(
            matches!(&results[1], Err(DatabaseError(SerializationFailure, _))),
            "Got {:?} instead",
            results
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    // This function uses a collect with side effects (spawning threads)
    // so clippy is wrong here
    #[allow(clippy::needless_collect)]
    fn mysql_nested_transaction_depth_commits_tracked_properly_on_serialization_failure() {
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::DatabaseError;
        use crate::*;
        use std::num::NonZeroU32;
        use std::sync::{Arc, Barrier};
        use std::thread;

        table! {
            #[sql_name = "mysql_nested_trans_depth_is_tracked_properly_on_commit_failure"]
            serialization_example {
                id -> Integer,
                class -> Integer,
            }
        }

        let conn = &mut crate::test_helpers::connection_no_transaction();

        sql_query(
            "DROP TABLE IF EXISTS mysql_nested_trans_depth_is_tracked_properly_on_commit_failure;",
        )
        .execute(conn)
        .unwrap();
        sql_query(
            r#"
            CREATE TABLE mysql_nested_trans_depth_is_tracked_properly_on_commit_failure (
                id INT AUTO_INCREMENT PRIMARY KEY,
                class INTEGER NOT NULL
            )
        "#,
        )
        .execute(conn)
        .unwrap();

        insert_into(serialization_example::table)
            .values(&vec![
                serialization_example::class.eq(1),
                serialization_example::class.eq(2),
            ])
            .execute(conn)
            .unwrap();

        let before_barrier = Arc::new(Barrier::new(2));
        let after_barrier = Arc::new(Barrier::new(2));

        let threads = (1..3)
            .map(|i| {
                let before_barrier = before_barrier.clone();
                let after_barrier = after_barrier.clone();
                thread::spawn(move || {
                    use crate::connection::transaction_manager::AnsiTransactionManager;
                    use crate::connection::transaction_manager::TransactionManager;
                    let conn = &mut crate::test_helpers::connection_no_transaction();
                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                    crate::sql_query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE").execute(conn)?;

                    let result =
                    conn.transaction(|conn| {
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                       conn.transaction(|conn| {
                            assert_eq!(NonZeroU32::new(2), <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                            let _ = serialization_example::table
                                .filter(serialization_example::class.eq(i))
                                .count()
                                .execute(conn)?;

                            let other_i = if i == 1 { 2 } else { 1 };
                            let q = insert_into(serialization_example::table)
                                .values(serialization_example::class.eq(other_i));
                            before_barrier.wait();

                            let r = q.execute(conn);
                            after_barrier.wait();
                            r
                        })
                    });

                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let second_trans_result = conn.transaction(|conn| crate::sql_query("SELECT 1").execute(conn));
                    assert!(second_trans_result.is_ok(), "Expected the thread connections to have been rolled back or committed, but second transaction exited with {:?}", second_trans_result);
                    result
                })
            })
            .collect::<Vec<_>>();
        let second_trans_result =
            conn.transaction(|conn| crate::sql_query("SELECT 1").execute(conn));
        assert!(second_trans_result.is_ok(), "Expected the main connection to have been rolled back or committed, but second transaction exited with {:?}", second_trans_result);

        let mut results = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();

        results.sort_by_key(|r| r.is_err());
        assert!(results[0].is_ok(), "Got {:?} instead", results);
        assert!(
            matches!(&results[1], Err(DatabaseError(SerializationFailure, _))),
            "Got {:?} instead",
            results
        );
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn sqlite_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::result::Error;
        use crate::*;
        use std::num::NonZeroU32;

        let conn = &mut crate::test_helpers::connection();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.transaction(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_commit(id INT UNIQUE INITIALLY DEFERRED)").execute(conn)?;
            sql_query("INSERT INTO deferred_commit VALUES(1)").execute(conn)?;
            let result = sql_query("INSERT INTO deferred_commit VALUES(1)").execute(conn);
            assert!(result.is_ok());
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    // regression test for #3470
    // crates.io depends on this behaviour
    #[test]
    #[cfg(feature = "postgres")]
    fn some_libpq_failures_are_recoverable_by_rolling_back_the_savepoint_only() {
        use crate::connection::{AnsiTransactionManager, TransactionManager};
        use crate::prelude::*;
        use crate::sql_query;

        crate::table! {
            rollback_test (id) {
                id -> Int4,
                value -> Int4,
            }
        }

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );

        let res = conn.transaction(|conn| {
            sql_query(
                "CREATE TABLE IF NOT EXISTS rollback_test (id INT PRIMARY KEY, value INT NOT NULL)",
            )
            .execute(conn)?;
            conn.transaction(|conn| {
                sql_query("SET TRANSACTION READ ONLY").execute(conn)?;
                crate::update(rollback_test::table)
                    .set(rollback_test::value.eq(0))
                    .execute(conn)
            })
            .map(|_| {
                panic!("Should use the `or_else` branch");
            })
            .or_else(|_| sql_query("SELECT 1").execute(conn))
            .map(|_| ())
        });
        assert!(res.is_ok());

        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn other_libpq_failures_are_not_recoverable_by_rolling_back_the_savepoint_only() {
        use crate::connection::{AnsiTransactionManager, TransactionManager};
        use crate::prelude::*;
        use crate::sql_query;
        use std::num::NonZeroU32;
        use std::sync::{Arc, Barrier};

        crate::table! {
            rollback_test2 (id) {
                id -> Int4,
                value -> Int4,
            }
        }
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();

        sql_query(
            "CREATE TABLE IF NOT EXISTS rollback_test2 (id INT PRIMARY KEY, value INT NOT NULL)",
        )
        .execute(conn)
        .unwrap();

        let start_barrier = Arc::new(Barrier::new(2));
        let commit_barrier = Arc::new(Barrier::new(2));

        let other_start_barrier = start_barrier.clone();
        let other_commit_barrier = commit_barrier.clone();

        let t1 = std::thread::spawn(move || {
            let conn = &mut crate::test_helpers::pg_connection_no_transaction();
            assert_eq!(
                None,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
            let r = conn.build_transaction().serializable().run::<_, crate::result::Error, _>(|conn| {
                assert_eq!(
                    NonZeroU32::new(1),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
                    ).transaction_depth().expect("Transaction depth")
                );
                rollback_test2::table.load::<(i32, i32)>(conn)?;
                crate::insert_into(rollback_test2::table)
                    .values((rollback_test2::id.eq(1), rollback_test2::value.eq(42)))
                    .execute(conn)?;
                let r = conn.transaction(|conn| {
                    assert_eq!(
                        NonZeroU32::new(2),
                        <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                            conn
                        ).transaction_depth().expect("Transaction depth")
                    );
                    start_barrier.wait();
                    commit_barrier.wait();
                    let r = rollback_test2::table.load::<(i32, i32)>(conn);
                    assert!(r.is_err());
                    Err::<(), _>(crate::result::Error::RollbackTransaction)
                });
                assert_eq!(
                    NonZeroU32::new(1),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
                    ).transaction_depth().expect("Transaction depth")
                );
                assert!(
                    matches!(r, Err(crate::result::Error::RollbackTransaction)),
                    "rollback failed (such errors should be ignored by transaction manager): {}",
                    r.unwrap_err()
                );
                let r = rollback_test2::table.load::<(i32, i32)>(conn);
                assert!(r.is_err());
                // fun fact: if hitting "commit" after receiving a serialization failure, PG
                // returns that the commit has succeeded, but in fact it was actually rolled back.
                // soo.. one should avoid doing that
                r
            });
            assert!(r.is_err());
            assert_eq!(
                None,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
        });

        let t2 = std::thread::spawn(move || {
            other_start_barrier.wait();
            let conn = &mut crate::test_helpers::pg_connection_no_transaction();
            assert_eq!(
                None,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
            let r = conn.build_transaction().serializable().run::<_, crate::result::Error, _>(|conn| {
                assert_eq!(
                    NonZeroU32::new(1),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
                    ).transaction_depth().expect("Transaction depth")
                );
                let _ = rollback_test2::table.load::<(i32, i32)>(conn)?;
                crate::insert_into(rollback_test2::table)
                    .values((rollback_test2::id.eq(23), rollback_test2::value.eq(42)))
                    .execute(conn)?;
                Ok(())
            });
            other_commit_barrier.wait();
            assert!(r.is_ok(), "{:?}", r.unwrap_err());
            assert_eq!(
                None,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
        });
        crate::sql_query("DELETE FROM rollback_test2")
            .execute(conn)
            .unwrap();
        t1.join().unwrap();
        t2.join().unwrap();
    }
}
