use crate::connection::commit_error_processor::{CommitErrorOutcome, CommitErrorProcessor};
use crate::connection::Connection;
use crate::result::{Error, QueryResult};
use std::borrow::Cow;

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
    status: TransactionManagerStatus,
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
    pub fn transaction_depth(&self) -> Result<i32, Error> {
        match self {
            TransactionManagerStatus::Valid(valid_status) => Ok(valid_status.transaction_depth()),
            TransactionManagerStatus::InError => Err(Error::BrokenTransaction),
        }
    }
}

/// Valid transaction status for the manager. Can return the current transaction depth
#[allow(missing_copy_implementations)]
#[derive(Debug, PartialEq, Eq, Default)]
pub struct ValidTransactionManagerStatus {
    transaction_depth: i32,
}

impl ValidTransactionManagerStatus {
    /// Return the current transaction depth
    pub fn transaction_depth(&self) -> i32 {
        self.transaction_depth
    }

    /// Update the transaction depth by adding the value of the `by` parameter if the `query` is
    /// `Ok(())`
    pub fn change_transaction_depth(&mut self, by: i32, query: QueryResult<()>) -> QueryResult<()> {
        if query.is_ok() {
            self.transaction_depth += by;
        }
        query
    }
}

/// The outcome of a transactional operation
struct OperationOutcome {
    /// The result of the operation
    result: QueryResult<()>,
    /// The way the state of the transaction manager should be updated
    update: ManagerUpdate,
}

enum ManagerUpdate {
    UpdateTransaction(TransactionUpdate),
    MarkManagerAsInError,
}

struct TransactionUpdate {
    update_condition: UpdateCondition,
    transaction_depth_change: TransactionDepthChange,
}

#[derive(Clone, Copy)]
enum TransactionDepthChange {
    IncreaseDepth,
    DecreaseDepth,
}

enum UpdateCondition {
    /// Only update the transaction depth if if the result was Ok
    AfterResultCheck,
    // Only update the transaction depth if the content of this variant is OK
    AfterCheck(QueryResult<()>),
    /// Always update the result
    Unconditionally,
}

impl AnsiTransactionManager {
    fn run_if_valid<Conn>(
        conn: &mut Conn,
        mut action: impl FnMut(&mut Conn, i32) -> Result<OperationOutcome, Error>,
    ) -> QueryResult<()>
    where
        Conn: Connection<TransactionManager = Self> + CommitErrorProcessor,
    {
        let transaction_depth = match &mut conn.transaction_state().status {
            TransactionManagerStatus::InError => return Err(Error::BrokenTransaction),
            TransactionManagerStatus::Valid(valid_status) => valid_status.transaction_depth(),
        };
        let OperationOutcome { result, update } = action(conn, transaction_depth)?;
        let status = &mut conn.transaction_state().status;
        match status {
            TransactionManagerStatus::InError => Err(Error::BrokenTransaction), // Unlikely, but the action actually updated the transaction state and broke it
            TransactionManagerStatus::Valid(valid_status) => match update {
                ManagerUpdate::UpdateTransaction(transaction_update) => {
                    let transaction_delta = match transaction_update.transaction_depth_change {
                        TransactionDepthChange::IncreaseDepth => 1,
                        TransactionDepthChange::DecreaseDepth => -1,
                    };
                    match transaction_update.update_condition {
                        UpdateCondition::AfterResultCheck => {
                            valid_status.change_transaction_depth(transaction_delta, result)
                        }
                        UpdateCondition::AfterCheck(result_to_check) => {
                            let _r = valid_status
                                .change_transaction_depth(transaction_delta, result_to_check);
                            result
                        }
                        UpdateCondition::Unconditionally => {
                            let _r =
                                valid_status.change_transaction_depth(transaction_delta, Ok(()));
                            result
                        }
                    }
                }
                ManagerUpdate::MarkManagerAsInError => {
                    *status = TransactionManagerStatus::InError;
                    result
                }
            },
        }
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
        AnsiTransactionManager::run_if_valid(
            conn,
            |conn, transaction_depth| match transaction_depth {
                0 => Ok(OperationOutcome {
                    result: conn.batch_execute(sql),
                    update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                        update_condition: UpdateCondition::AfterResultCheck,
                        transaction_depth_change: TransactionDepthChange::IncreaseDepth,
                    }),
                }),
                depth if depth > 0 => Err(Error::AlreadyInTransaction),
                _depth => panic!("Transaction depth < 0"),
            },
        )
    }
}

impl<Conn> TransactionManager<Conn> for AnsiTransactionManager
where
    Conn: Connection<TransactionManager = Self> + CommitErrorProcessor,
{
    type TransactionStateData = Self;

    fn begin_transaction(conn: &mut Conn) -> QueryResult<()> {
        AnsiTransactionManager::run_if_valid(conn, |conn, transaction_depth| {
            let start_transaction_sql = match transaction_depth {
                i32::MIN..=-1 => panic!("Transaction depth < 0"),
                0 => Cow::from("BEGIN"),
                1..=i32::MAX => {
                    Cow::from(format!("SAVEPOINT diesel_savepoint_{}", transaction_depth))
                }
            };
            let result = conn.batch_execute(&*start_transaction_sql);
            Ok(OperationOutcome {
                result,
                update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                    update_condition: UpdateCondition::AfterResultCheck,
                    transaction_depth_change: TransactionDepthChange::IncreaseDepth,
                }),
            })
        })
    }

    fn rollback_transaction(conn: &mut Conn) -> QueryResult<()> {
        AnsiTransactionManager::run_if_valid(conn, |conn, transaction_depth| {
            let rollback_sql = match transaction_depth {
                i32::MIN..=-1 => panic!("Transaction depth < 0"),
                0 => return Err(Error::NotInTransaction),
                1 => Cow::from("ROLLBACK"),
                2..=i32::MAX => Cow::from(format!(
                    "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                    transaction_depth - 1
                )),
            };
            let result = conn.batch_execute(&*rollback_sql);
            Ok(OperationOutcome {
                result,
                update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                    update_condition: UpdateCondition::AfterResultCheck,
                    transaction_depth_change: TransactionDepthChange::DecreaseDepth,
                }),
            })
        })
    }

    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted. If the rollback succeeds,
    /// the original error will be returned, otherwise the error generated by the rollback
    /// will be returned. In the second case the connection should be considered broken
    /// as it contains a uncommitted unabortable open transaction.
    fn commit_transaction(conn: &mut Conn) -> QueryResult<()> {
        AnsiTransactionManager::run_if_valid(conn, |conn, transaction_depth| {
            let transaction_depth_change = TransactionDepthChange::DecreaseDepth;
            let (commit_sql, rollback_sql) = match transaction_depth {
                i32::MIN..=-1 => panic!("Transaction depth < 0"),
                0 => return Err(Error::NotInTransaction),
                1 => (Cow::from("COMMIT"), Cow::from("ROLLBACK")),
                2..=i32::MAX => (
                    Cow::from(format!(
                        "RELEASE SAVEPOINT diesel_savepoint_{}",
                        transaction_depth - 1
                    )),
                    Cow::from(format!(
                        "ROLLBACK TO SAVEPOINT diesel_savepoint_{}",
                        transaction_depth - 1
                    )),
                ),
            };
            match conn.batch_execute(&*commit_sql) {
                result @ Ok(()) => Ok(OperationOutcome {
                    result,
                    update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                        update_condition: UpdateCondition::Unconditionally,
                        transaction_depth_change,
                    }),
                }),
                Err(error) => match conn.process_commit_error(transaction_depth, error) {
                    CommitErrorOutcome::RollbackAndThrow(error) => {
                        let rollback_result = conn.batch_execute(&*rollback_sql);
                        Ok(OperationOutcome {
                            result: Err(error),
                            update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                                update_condition: UpdateCondition::AfterCheck(rollback_result),
                                transaction_depth_change,
                            }),
                        })
                    }
                    CommitErrorOutcome::Throw(error) => Ok(OperationOutcome {
                        result: Err(error),
                        update: ManagerUpdate::UpdateTransaction(TransactionUpdate {
                            update_condition: UpdateCondition::Unconditionally,
                            transaction_depth_change,
                        }),
                    }),
                    CommitErrorOutcome::ThrowAndMarkManagerAsBroken(error) => {
                        Ok(OperationOutcome {
                            result: Err(error),
                            update: ManagerUpdate::MarkManagerAsInError,
                        })
                    }
                },
            }
        })
    }

    fn transaction_manager_status_mut(conn: &mut Conn) -> &mut TransactionManagerStatus {
        &mut conn.transaction_state().status
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "postgres")]
    macro_rules! matches {
        ($expression:expr, $( $pattern:pat )|+ $( if $guard: expr )?) => {
            match $expression {
                $( $pattern )|+ $( if $guard )? => true,
                _ => false
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = AnsiTransactionManager::rollback_transaction(conn);
        assert!(matches!(result, Err(Error::NotInTransaction)))
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn postgres_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.transaction(|conn| {
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let _result = conn.transaction(|conn| {
            assert_eq!(
                1,
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
            0,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn nested_postgres_transaction_is_rolled_back_upon_syntax_error() {
        use crate::connection::transaction_manager::AnsiTransactionManager;
        use crate::connection::transaction_manager::TransactionManager;
        use crate::pg::connection::raw::PgTransactionStatus;
        use crate::*;
        let conn = &mut crate::test_helpers::pg_connection_no_transaction();
        assert_eq!(
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let result = conn.build_transaction().run(|conn| {
                assert_eq!(
                    2,
                    <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                        conn
            ).transaction_depth().expect("Transaction depth")
                );
                sql_query("SELECT_SYNTAX_ERROR 1").execute(conn)
            });
            assert!(result.is_err());
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result = conn.transaction(|conn| {
            assert_eq!(
                1,
                <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            let result = conn.transaction(|conn| {
                assert_eq!(
                    2,
                    <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                        conn
            ).transaction_depth().expect("Transaction depth")
                );
                // In MySQL, a syntax error does not break the transaction block
                sql_query("SELECT_SYNTAX_ERROR 1").execute(conn)
            });
            assert!(result.is_err());
            assert_eq!(
                1,
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
            0,
            <AnsiTransactionManager as TransactionManager<MysqlConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn transaction_depth_is_tracked_properly_on_commit_failure() {
        use crate::result::DatabaseErrorKind::SerializationFailure;
        use crate::result::Error::DatabaseError;
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

        let barrier = Arc::new(Barrier::new(2));
        let threads = (1..3)
            .map(|i| {
                let barrier = barrier.clone();
                thread::spawn(move || {
                    use crate::connection::transaction_manager::AnsiTransactionManager;
                    use crate::connection::transaction_manager::TransactionManager;
                    let conn = &mut crate::test_helpers::pg_connection_no_transaction();
                    assert_eq!(0, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                    let result =
                    conn.build_transaction().serializable().run(|conn| {
                        assert_eq!(1, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));

                        let _ = serialization_example::table
                            .filter(serialization_example::class.eq(i))
                            .count()
                            .execute(conn)?;

                        barrier.wait();

                        let other_i = if i == 1 { 2 } else { 1 };
                        insert_into(serialization_example::table)
                            .values(serialization_example::class.eq(other_i))
                            .execute(conn)
                    });

                    assert_eq!(0, <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(conn).transaction_depth().expect("Transaction depth"));
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
            results[1],
            Err(DatabaseError(SerializationFailure, _))
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
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
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
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.build_transaction().run(|conn| {
            assert_eq!(
                1,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("DROP TABLE IF EXISTS deferred_constraint_nested_commit").execute(conn)?;
            sql_query("CREATE TABLE deferred_constraint_nested_commit(id INT UNIQUE INITIALLY DEFERRED)").execute(conn)?;
            let inner_result: Result<_, Error> = conn.build_transaction().run(|conn| {
                assert_eq!(
                    2,
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
                1,
                <AnsiTransactionManager as TransactionManager<PgConnection>>::transaction_manager_status_mut(
                    conn
            ).transaction_depth().expect("Transaction depth")
            );
            sql_query("INSERT INTO deferred_constraint_nested_commit VALUES(1)").execute(conn)
        });
        assert_eq!(
            0,
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
            0,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
        let result: Result<_, Error> = conn.transaction(|conn| {
            assert_eq!(
                1,
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
            0,
            <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
                conn
            ).transaction_depth().expect("Transaction depth")
        );
    }
}
