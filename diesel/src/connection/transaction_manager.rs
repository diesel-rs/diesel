use crate::connection::commit_error_processor::{CommitErrorOutcome, CommitErrorProcessor};
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
    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus;
}

/// An implementation of `TransactionManager` which can be used for backends
/// which use ANSI standard syntax for savepoints such as SQLite and PostgreSQL.
#[allow(missing_debug_implementations, missing_copy_implementations)]
#[derive(Default)]
pub struct AnsiTransactionManager {
    pub(crate) status: TransactionManagerStatus,
}

/// Status of the transaction manager
#[derive(Debug, PartialEq, Eq)]
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
    /// [`Error::BrokenTransaction`] if the transaction manager is in error.
    pub fn transaction_depth(&self) -> QueryResult<Option<NonZeroU32>> {
        match self {
            TransactionManagerStatus::Valid(valid_status) => Ok(valid_status.transaction_depth()),
            TransactionManagerStatus::InError => Err(Error::BrokenTransaction),
        }
    }

    fn transaction_state(&mut self) -> QueryResult<&mut ValidTransactionManagerStatus> {
        match self {
            TransactionManagerStatus::Valid(valid_status) => Ok(valid_status),
            TransactionManagerStatus::InError => Err(Error::BrokenTransaction),
        }
    }
}

/// Valid transaction status for the manager. Can return the current transaction depth
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Default)]
pub struct ValidTransactionManagerStatus {
    pub(super) transaction_depth: Option<NonZeroU32>,
}

impl ValidTransactionManagerStatus {
    /// Return the current transaction depth
    ///
    /// This value is `None` if no current transaction is running
    /// otherwise the number of nested transactions is returned.
    pub fn transaction_depth(&self) -> Option<NonZeroU32> {
        self.transaction_depth
    }

    /// Update the transaction depth by adding the value of the `transaction_depth_change` parameter if the `query` is
    /// `Ok(())`
    pub fn change_transaction_depth(
        &mut self,
        transaction_depth_change: TransactionDepthChange,
        query: QueryResult<()>,
    ) -> QueryResult<()> {
        if query.is_ok() {
            match (&mut self.transaction_depth, transaction_depth_change) {
                (Some(depth), TransactionDepthChange::IncreaseDepth) => {
                    // This is always `Some(_)`
                    self.transaction_depth = NonZeroU32::new(depth.get().saturating_add(1));
                }
                (Some(depth), TransactionDepthChange::DecreaseDepth) => {
                    // This sets `transaction_depth` to `None` as soon as we reach zero
                    self.transaction_depth = NonZeroU32::new(depth.get().saturating_sub(1));
                }
                (None, TransactionDepthChange::IncreaseDepth) => {
                    self.transaction_depth = NonZeroU32::new(1);
                }
                (None, TransactionDepthChange::DecreaseDepth) => {
                    // We screwed up something somewhere
                    // we cannot decrease the transaction count if
                    // we are not inside a transaction
                    return Err(Error::NotInTransaction);
                }
            }
        }
        query
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
        Conn: Connection<TransactionManager = Self> + CommitErrorProcessor,
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
        Conn: Connection<TransactionManager = Self> + CommitErrorProcessor,
    {
        let state = Self::get_transaction_state(conn)?;
        match state.transaction_depth() {
            None => {
                let res = conn.batch_execute(sql);
                let state = Self::get_transaction_state(conn)?;
                state.change_transaction_depth(TransactionDepthChange::IncreaseDepth, res)
            }
            Some(_depth) => Err(Error::AlreadyInTransaction),
        }
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager
where
    Conn: Connection<TransactionManager = Self> + CommitErrorProcessor,
{
    type TransactionStateData = Self;

    fn begin_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let start_transaction_sql = match transaction_state.transaction_depth() {
            None => Cow::from("BEGIN"),
            Some(transaction_depth) => {
                Cow::from(format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
            }
        };
        let result = conn.batch_execute(&*start_transaction_sql);
        let state = Self::get_transaction_state(conn)?;
        state.change_transaction_depth(TransactionDepthChange::IncreaseDepth, result)
    }

    fn rollback_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let rollback_sql = match transaction_state.transaction_depth() {
            None => return Err(Error::NotInTransaction),
            Some(transaction_depth) if transaction_depth.get() == 1 => Cow::from("ROLLBACK"),
            Some(transaction_depth) => Cow::from(format!(
                "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                transaction_depth.get() - 1
            )),
        };
        let result = conn.batch_execute(&*rollback_sql);
        let state = Self::get_transaction_state(conn)?;
        state.change_transaction_depth(TransactionDepthChange::DecreaseDepth, result)
    }

    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be returned. In the second case the connection should be considered broken
    /// as it contains a uncommitted unabortable open transaction.
    fn commit_transaction(conn: &mut Conn) -> QueryResult<()> {
        let transaction_state = Self::get_transaction_state(conn)?;
        let transaction_depth = transaction_state.transaction_depth();
        let (commit_sql, rollback_sql) = match transaction_depth {
            None => return Err(Error::NotInTransaction),
            Some(transaction_depth) if transaction_depth.get() == 1 => {
                (Cow::from("COMMIT"), Cow::from("ROLLBACK"))
            }
            Some(transaction_depth) => (
                Cow::from(format!(
                    "RELEASE SAVEPOINT diesel_savepoint_{}",
                    transaction_depth.get() - 1
                )),
                Cow::from(format!(
                    "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                    transaction_depth.get() - 1
                )),
            ),
        };
        let res = conn.batch_execute(&*commit_sql);
        let state = Self::get_transaction_state(conn)?;
        match res {
            Ok(()) => {
                // commit succeeded, so we just decrease the transaction depth
                // and we are done
                state.change_transaction_depth(TransactionDepthChange::DecreaseDepth, res)
            }
            Err(error) => {
                let commit_error_outcome = conn.process_commit_error(error);
                let state = Self::get_transaction_state(conn)?;
                // commit failed so we call the error processor to get a information out
                // what should be done
                match commit_error_outcome {
                    CommitErrorOutcome::RollbackAndThrow(error) => {
                        // We should try to rollback the transaction here
                        let rollback_result = conn.batch_execute(&*rollback_sql);
                        let state = Self::get_transaction_state(conn)?;
                        let rollback_result = state.change_transaction_depth(
                            TransactionDepthChange::DecreaseDepth,
                            rollback_result,
                        );
                        Err(Error::CommitTransactionFailed {
                            commit_error: Box::new(error),
                            rollback_result: Box::new(rollback_result),
                        })
                    }
                    CommitErrorOutcome::Throw(error) => {
                        // The error processor indicated that we just
                        // need to decrease the transaction depth and return the original error
                        let _ = state.change_transaction_depth(
                            TransactionDepthChange::DecreaseDepth,
                            Ok(()),
                        );
                        Err(Error::CommitTransactionFailed {
                            commit_error: Box::new(error),
                            rollback_result: Box::new(Ok(())),
                        })
                    }
                    CommitErrorOutcome::ThrowAndMarkManagerAsBroken(error) => {
                        // The connection contains an unrecoverable broken transaction
                        // so mark the transaction state as broken and return the error
                        *Self::transaction_manager_status_mut(conn) =
                            TransactionManagerStatus::InError;
                        Err(Error::CommitTransactionFailed {
                            commit_error: Box::new(error),
                            rollback_result: Box::new(Err(Error::BrokenTransaction)),
                        })
                    }
                }
            }
        }
    }

    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus {
        &mut conn.transaction_state().status
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU32;

    // Mock connection. Uses the Postgres backend for type signatures.
    // This is really because implementing a new backend is pain
    mod mock {
        use crate::connection::commit_error_processor::{CommitErrorOutcome, CommitErrorProcessor};
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::{
            Connection, ConnectionGatWorkaround, SimpleConnection, TransactionManager,
        };
        use crate::expression::QueryMetadata;
        use crate::query_builder::{AsQuery, QueryFragment, QueryId};
        use crate::result::{Error, QueryResult};
        use crate::test_helpers::TestConnection;

        pub(crate) struct MockConnection {
            pub next_result: Option<QueryResult<usize>>,
            pub next_batch_execute_result: Option<QueryResult<()>>,
            pub broken: bool,
            transaction_state: AnsiTransactionManager,
        }

        impl SimpleConnection for MockConnection {
            fn batch_execute(&mut self, _query: &str) -> QueryResult<()> {
                self.next_batch_execute_result
                    .take()
                    .expect("No next result")
            }
        }

        impl<'conn, 'query>
            ConnectionGatWorkaround<'conn, 'query, <TestConnection as Connection>::Backend>
            for MockConnection
        {
            type Cursor = <TestConnection as ConnectionGatWorkaround<
                'conn,
                'query,
                <TestConnection as Connection>::Backend,
            >>::Cursor;

            type Row = <TestConnection as ConnectionGatWorkaround<
                'conn,
                'query,
                <TestConnection as Connection>::Backend,
            >>::Row;
        }

        impl CommitErrorProcessor for MockConnection {
            fn process_commit_error(&self, error: Error) -> CommitErrorOutcome {
                if self.broken {
                    CommitErrorOutcome::ThrowAndMarkManagerAsBroken(error)
                } else {
                    CommitErrorOutcome::Throw(error)
                }
            }
        }

        impl Connection for MockConnection {
            type Backend = <TestConnection as Connection>::Backend;

            type TransactionManager = AnsiTransactionManager;

            fn establish(_database_url: &str) -> crate::ConnectionResult<Self> {
                Ok(Self {
                    next_result: None,
                    next_batch_execute_result: None,
                    broken: false,
                    transaction_state: AnsiTransactionManager::default(),
                })
            }

            fn execute(&mut self, _query: &str) -> QueryResult<usize> {
                self.next_result.take().expect("No next result")
            }

            fn load<'conn, 'query, T>(
                &'conn mut self,
                _source: T,
            ) -> QueryResult<<Self as ConnectionGatWorkaround<'conn, 'query, Self::Backend>>::Cursor>
            where
                T: AsQuery,
                T::Query: QueryFragment<Self::Backend> + QueryId + 'query,
                Self::Backend: QueryMetadata<T::SqlType>,
            {
                unimplemented!()
            }

            fn execute_returning_count<T>(&mut self, _source: &T) -> QueryResult<usize>
            where
                T: crate::query_builder::QueryFragment<Self::Backend>
                    + crate::query_builder::QueryId,
            {
                self.next_result.take().expect("No next result")
            }

            fn transaction_state(
                &mut self,
            ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData
            {
                &mut self.transaction_state
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
        conn.next_batch_execute_result = Some(Ok(()));
        let result = conn.transaction(|conn| {
            conn.next_result = Some(Ok(1));
            let query_result = sql_query("SELECT 1").execute(conn);
            assert!(query_result.is_ok());
            conn.broken = true;
            // Set result for COMMIT attempt
            conn.next_batch_execute_result = Some(Err(Error::DatabaseError(
                DatabaseErrorKind::Unknown,
                Box::new("whatever".to_string()),
            )));
            Ok(())
        });
        assert!(matches!(
            result,
            Err(Error::CommitTransactionFailed{commit_error, ..}) if matches!(&*commit_error, Error::DatabaseError(DatabaseErrorKind::Unknown, _))
        ));
        assert_eq!(
            *<AnsiTransactionManager as TransactionManager<mock::MockConnection>>::transaction_manager_status_mut(
                &mut conn),
            TransactionManagerStatus::InError
        );
        // Ensure the transaction manager is unusable
        let result = conn.transaction(|_conn| Ok(()));
        assert!(matches!(result, Err(Error::BrokenTransaction)))
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn postgres_transaction_is_rolled_back_upon_syntax_error() {
        use std::num::NonZeroU32;

        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
                ).transaction_depth().expect("Transaction depth")
            );
            // In Postgres, a syntax error breaks the transaction block
            let query_result = sql_query("SELECT_SYNTAX_ERROR 1").execute(conn);
            assert!(query_result.is_err());
            assert_eq!(
                PgTransactionStatus::InError,
                conn.raw_connection.transaction_status()
            );
            query_result
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
    }

    #[test]
    #[cfg(feature = "mysql")]
    fn mysql_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::*;
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
    #[cfg(feature = "postgres")]
    fn nested_postgres_transaction_is_rolled_back_upon_syntax_error() {
        use std::num::NonZeroU32;

        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let result = conn.build_transaction().run(|conn| {
                assert_eq!(
                    NonZeroU32::new(2),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
            ).transaction_depth().expect("Transaction depth")
                );
                sql_query("SELECT_SYNTAX_ERROR 1").execute(conn)
            });
            assert!(result.is_err());
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let query_result = sql_query("SELECT 1").execute(conn);
            assert!(query_result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            query_result
        });
        assert!(result.is_ok());
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
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
    #[cfg(feature = "postgres")]
    fn transaction_depth_is_tracked_properly_on_commit_failure() {
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::{self, DatabaseError};
        use crate::*;
        use std::sync::{Arc, Barrier};
        use std::thread;

        table! {
            #[sql_name = "transaction_depth_is_tracked_properly_on_commit_failure"]
            serialization_example {
                id -> Serial,
                class -> Integer,
            }
        }

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();

        sql_query("DROP TABLE IF EXISTS transaction_depth_is_tracked_properly_on_commit_failure;")
            .execute(conn)
            .unwrap();
        sql_query(
            r#"
            CREATE TABLE transaction_depth_is_tracked_properly_on_commit_failure (
                id SERIAL PRIMARY KEY,
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
                    let conn = &mut crate::test_helpers::pg_connection_no_transaction();
                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let result =
                    conn.build_transaction().serializable().run(|conn| {
                        assert_eq!(NonZeroU32::new(1), <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

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

                    assert_eq!(None, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
                    result
                })
            })
            .collect::<Vec<_>>();

        let mut results = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>();

        results.sort_by_key(|r| r.is_err());

        assert!(matches!(results[0], Ok(_)));
        assert!(matches!(
            &results[1],
            Err(Error::CommitTransactionFailed {
                ref commit_error, ..
            }) if matches!(&**commit_error, DatabaseError(SerializationFailure, _))
        ));
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn postgres_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_constraint_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_constraint_commit(id INT UNIQUE INITIALLY DEFERRED)")
                .execute(conn)?;
            sql_query("INSERT INTO deferred_constraint_commit VALUES(1)").execute(conn)?;
            let result =
                sql_query("INSERT INTO deferred_constraint_commit VALUES(1)").execute(conn);
            assert!(result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn postgres_transaction_is_rolled_back_upon_deferred_trigger_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_trigger_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_trigger_commit(id INT UNIQUE INITIALLY DEFERRED)")
                .execute(conn)?;
            sql_query(
                r#"
                    CREATE OR REPLACE FUNCTION transaction_depth_blow_up()
                        RETURNS trigger
                        LANGUAGE plpgsql
                        AS $$
                    DECLARE
                    BEGIN
                        IF NEW.value = 42 THEN
                            RAISE EXCEPTION 'Transaction kaboom';
                        END IF;
                    RETURN NEW;

                    END;$$;
                "#,
            )
            .execute(conn)?;

            sql_query(
                r#"
                    CREATE CONSTRAINT TRIGGER transaction_depth_trigger
                        AFTER INSERT ON "deferred_trigger_commit"
                        DEFERRABLE INITIALLY DEFERRED
                        FOR EACH ROW
                        EXECUTE PROCEDURE transaction_depth_blow_up()
            "#,
            )
            .execute(conn)?;
            let result = sql_query("INSERT INTO deferred_trigger_commit VALUES(42)").execute(conn);
            assert!(result.is_ok());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn nested_postgres_transaction_is_rolled_back_upon_deferred_trigger_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_trigger_nested_commit").execute(conn)?;
            sql_query(
                "CREATE TABLE deferred_trigger_nested_commit(id INT UNIQUE INITIALLY DEFERRED)",
            )
            .execute(conn)?;
            sql_query(
                r#"
                    CREATE OR REPLACE FUNCTION transaction_depth_blow_up()
                        RETURNS trigger
                        LANGUAGE plpgsql
                        AS $$
                    DECLARE
                    BEGIN
                        IF NEW.value = 42 THEN
                            RAISE EXCEPTION 'Transaction kaboom';
                        END IF;
                    RETURN NEW;

                    END;$$;
                "#,
            )
            .execute(conn)?;

            sql_query(
                r#"
                    CREATE CONSTRAINT TRIGGER transaction_depth_trigger
                        AFTER INSERT ON "deferred_trigger_nested_commit"
                        DEFERRABLE INITIALLY DEFERRED
                        FOR EACH ROW
                        EXECUTE PROCEDURE transaction_depth_blow_up()
            "#,
            )
            .execute(conn)?;
            let inner_result: Result<_, Error> = conn.build_transaction().run(|conn| {
                let result = sql_query("INSERT INTO deferred_trigger_nested_commit VALUES(42)")
                    .execute(conn);
                assert!(result.is_ok());
                Ok(())
            });
            assert!(inner_result.is_err());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            Ok(())
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_ok(), "Expected success, got {:?}", result);
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn nested_postgres_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::result::Error;
        use crate::*;

        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_constraint_nested_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_constraint_nested_commit(id INT UNIQUE INITIALLY DEFERRED)").execute(conn)?;
            let inner_result: Result<_, Error> = conn.build_transaction().run(|conn| {
                assert_eq!(
                    NonZeroU32::new(2),
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
                    ).transaction_depth().expect("Transaction depth")
                );
                sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn)?;
                let result = sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn);
                assert!(result.is_ok());
                Ok(())
            });
            assert!(inner_result.is_err());
            assert_eq!(
                PgTransactionStatus::InTransaction,
                conn.raw_connection.transaction_status()
            );
            assert_eq!(
                NonZeroU32::new(1),
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn)
        });
        assert_eq!(
            None,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        assert_eq!(
            PgTransactionStatus::Idle,
            conn.raw_connection.transaction_status()
        );
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn sqlite_transaction_is_rolled_back_upon_deferred_constraint_failure() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::result::Error;
        use crate::*;
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
}
