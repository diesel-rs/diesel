use super::SqliteConnection;
use super::authorizer::{AuthorizerContext, AuthorizerDecision};
use crate::connection::{AnsiTransactionManager, Connection, SimpleConnection, TransactionManager};
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::result::{Error, QueryResult};
use crate::sqlite::Sqlite;
use crate::test_helpers::connection;
use crate::{ExpressionMethods, QueryDsl, QueryableByName, RunQueryDsl};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn depth(connection: &mut SqliteConnection) -> u32 {
    <AnsiTransactionManager as TransactionManager<SqliteConnection>>::transaction_manager_status_mut(
        connection,
    )
    .transaction_depth()
    .unwrap()
    .map_or(0, core::num::NonZeroU32::get)
}

/// Transaction control statements have no query DSL, so their SQL text is the subject of these tests.
fn execute(connection: &mut SqliteConnection, batch: bool, sql: &str) -> QueryResult<()> {
    if batch {
        connection.batch_execute(sql)
    } else {
        crate::sql_query(sql).execute(connection).map(|_| ())
    }
}

fn commit(connection: &mut SqliteConnection) -> QueryResult<()> {
    <AnsiTransactionManager as TransactionManager<SqliteConnection>>::commit_transaction(connection)
}

/// A statement that Diesel caches by its SQL text.
struct CachedControl(&'static str);

impl QueryId for CachedControl {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl QueryFragment<Sqlite> for CachedControl {
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        pass.push_sql(self.0);
        Ok(())
    }
}

/// Counts the savepoint statements the authorizer sees, ignoring them while `ignore` is set.
fn counting_policy(
    count: &Arc<AtomicUsize>,
    ignore: &Arc<AtomicBool>,
) -> impl FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send + 'static {
    let (count, ignore) = (Arc::clone(count), Arc::clone(ignore));
    move |context| match context {
        AuthorizerContext::Savepoint(_) => {
            count.fetch_add(1, Ordering::Relaxed);
            if ignore.load(Ordering::Relaxed) {
                AuthorizerDecision::Ignore
            } else {
                AuthorizerDecision::Allow
            }
        }
        _ => AuthorizerDecision::Allow,
    }
}

#[diesel_test_helper::test]
fn text_controls_track_depth_on_both_execution_paths() {
    for batch in [false, true] {
        let connection = &mut connection();
        for (sql, expected_depth) in [
            ("BEGIN", 1),
            ("SAVEPOINT x", 2),
            ("SAVEPOINT y", 3),
            ("ROLLBACK TO x", 2),
            ("RELEASE x", 1),
            ("END", 0),
            ("SAVEPOINT x", 1),
            ("SAVEPOINT X", 2),
            ("ROLLBACK TO x", 2),
            ("RELEASE x", 1),
            ("RELEASE X", 0),
            ("BEGIN IMMEDIATE", 1),
            ("COMMIT", 0),
            ("BEGIN EXCLUSIVE", 1),
            ("ROLLBACK", 0),
            ("SAVEPOINT \"odd \"\" name\"", 1),
            ("RELEASE \"odd \"\" name\"", 0),
        ] {
            execute(connection, batch, sql).unwrap();
            assert_eq!(depth(connection), expected_depth, "{sql}, batch: {batch}");
            assert_eq!(connection.in_transaction(), expected_depth != 0, "{sql}");
        }
    }
}

#[diesel_test_helper::test]
fn managed_transactions_compose_with_text_controls() {
    let connection = &mut connection();
    connection.batch_execute("BEGIN").unwrap();
    connection
        .transaction::<_, Error, _>(|connection| {
            assert_eq!(depth(connection), 2);
            Ok(())
        })
        .unwrap();
    assert_eq!(depth(connection), 1);
    connection.batch_execute("ROLLBACK").unwrap();

    connection.batch_execute("SAVEPOINT x").unwrap();
    commit(connection).unwrap();
    assert_eq!(depth(connection), 0);
    assert!(!connection.in_transaction());

    connection.batch_execute("BEGIN; SAVEPOINT x").unwrap();
    commit(connection).unwrap();
    assert_eq!(depth(connection), 1);
    assert!(connection.batch_execute("RELEASE x").is_err());
    connection.batch_execute("ROLLBACK").unwrap();
    assert_eq!(depth(connection), 0);
}

#[diesel_test_helper::test]
fn failed_controls_keep_the_open_frames() {
    for batch in [false, true] {
        let connection = &mut connection();
        execute(connection, batch, "BEGIN").unwrap();
        execute(connection, batch, "SAVEPOINT x").unwrap();
        for sql in ["BEGIN", "RELEASE missing", "ROLLBACK TO missing"] {
            assert!(execute(connection, batch, sql).is_err(), "{sql}");
            assert_eq!(depth(connection), 2, "{sql}");
        }
    }

    let connection = &mut connection();
    assert!(
        connection
            .batch_execute("BEGIN; SAVEPOINT x; RELEASE missing; COMMIT")
            .is_err()
    );
    assert_eq!(depth(connection), 2);
    assert!(connection.in_transaction());
}

#[diesel_test_helper::test]
fn explained_ignored_and_denied_controls_open_nothing() {
    for batch in [false, true] {
        let connection = &mut connection();
        for sql in ["EXPLAIN BEGIN", "EXPLAIN SAVEPOINT x"] {
            execute(connection, batch, sql).unwrap();
            assert_eq!(depth(connection), 0, "{sql}");
            assert!(!connection.in_transaction(), "{sql}");
        }

        for decision in [AuthorizerDecision::Ignore, AuthorizerDecision::Deny] {
            connection.on_authorize(move |context| match context {
                AuthorizerContext::Savepoint(_) => decision,
                _ => AuthorizerDecision::Allow,
            });
            let result = execute(connection, batch, "SAVEPOINT x");
            assert_eq!(result.is_ok(), decision == AuthorizerDecision::Ignore);
            assert_eq!(depth(connection), 0, "{decision:?}");
            assert!(!connection.in_transaction(), "{decision:?}");
        }
    }
}

#[diesel_test_helper::test]
fn cached_controls_apply_on_every_execution() {
    let connection = &mut connection();
    let count = Arc::new(AtomicUsize::new(0));
    let ignore = Arc::new(AtomicBool::new(false));
    connection.on_authorize(counting_policy(&count, &ignore));

    for expected_depth in [1, 2] {
        connection
            .execute_returning_count(&CachedControl("SAVEPOINT x"))
            .unwrap();
        assert_eq!(depth(connection), expected_depth);
    }
    assert_eq!(count.load(Ordering::Relaxed), 1, "prepared once");

    // Installing the authorizer again expires the cached statement without
    // evicting it, so SQLite recompiles it inside the next step.
    ignore.store(true, Ordering::Relaxed);
    connection
        .raw_connection
        .set_authorizer(counting_policy(&count, &ignore));
    connection
        .execute_returning_count(&CachedControl("SAVEPOINT x"))
        .unwrap();
    assert_eq!(count.load(Ordering::Relaxed), 2, "recompiled once");
    assert_eq!(depth(connection), 2, "the ignored savepoint opens nothing");

    ignore.store(false, Ordering::Relaxed);
    connection.remove_authorizer();
    connection
        .execute_returning_count(&CachedControl("SAVEPOINT x"))
        .unwrap();
    assert_eq!(depth(connection), 3);
    connection.batch_execute("ROLLBACK").unwrap();
    assert_eq!(depth(connection), 0);
}

#[diesel_test_helper::test]
fn loading_a_control_statement_completes_it() {
    #[derive(QueryableByName)]
    struct Value {
        #[diesel(sql_type = crate::sql_types::Integer)]
        value: i32,
    }

    let connection = &mut connection();
    // Transaction control has no query DSL.
    let rows = crate::sql_query("SAVEPOINT x")
        .load::<Value>(connection)
        .unwrap();
    assert!(rows.iter().all(|row| row.value == 0));
    assert_eq!(depth(connection), 1);
}

#[diesel_test_helper::test]
fn managed_savepoints_avoid_names_of_open_savepoints() {
    for raw_name in ["diesel_savepoint_1", "DIESEL_SAVEPOINT_1"] {
        let connection = &mut connection();
        connection
            .batch_execute(&alloc::format!("SAVEPOINT {raw_name}"))
            .unwrap();
        connection
            .transaction::<_, Error, _>(|connection| {
                assert_eq!(depth(connection), 2);
                Ok(())
            })
            .unwrap();
        assert_eq!(depth(connection), 1);
        let result = connection.transaction::<(), _, _>(|connection| {
            assert_eq!(depth(connection), 2);
            Err(Error::RollbackTransaction)
        });
        assert_eq!(result, Err(Error::RollbackTransaction));
        assert_eq!(depth(connection), 1);
        connection
            .batch_execute("ROLLBACK TO diesel_savepoint_1")
            .unwrap();
        connection
            .batch_execute("RELEASE diesel_savepoint_1")
            .unwrap();
        assert_eq!(depth(connection), 0);
    }
}

#[diesel_test_helper::test]
fn managed_scopes_close_their_own_frame_under_a_shadowing_name() {
    let connection = &mut connection();
    connection.batch_execute("SAVEPOINT a").unwrap();
    connection
        .transaction::<_, Error, _>(|connection| {
            // The managed savepoint is `diesel_savepoint_1`, now shadowed twice.
            connection
                .batch_execute("SAVEPOINT diesel_savepoint_1; SAVEPOINT diesel_savepoint_1")
                .unwrap();
            assert_eq!(depth(connection), 4);
            Ok(())
        })
        .unwrap();
    assert_eq!(depth(connection), 1);

    let result = connection.transaction::<(), _, _>(|connection| {
        connection.batch_execute("SAVEPOINT diesel_savepoint_1")?;
        Err(Error::RollbackTransaction)
    });
    assert_eq!(result, Err(Error::RollbackTransaction));
    assert_eq!(depth(connection), 1);
    connection.batch_execute("RELEASE a").unwrap();
    assert_eq!(depth(connection), 0);
}

#[diesel_test_helper::test]
fn managed_rollback_closes_its_savepoint() {
    crate::table! {
        kept (id) {
            id -> Integer,
        }
    }

    let connection = &mut connection();
    connection
        .batch_execute("CREATE TABLE kept (id INTEGER PRIMARY KEY)")
        .unwrap();
    connection
        .transaction::<_, Error, _>(|connection| {
            crate::insert_into(kept::table)
                .values(kept::id.eq(1))
                .execute(connection)?;
            let inner = connection.transaction::<(), _, _>(|connection| {
                crate::insert_into(kept::table)
                    .values(kept::id.eq(2))
                    .execute(connection)?;
                Err(Error::RollbackTransaction)
            });
            assert_eq!(inner, Err(Error::RollbackTransaction));
            assert_eq!(depth(connection), 1);
            assert!(
                connection
                    .batch_execute("ROLLBACK TO diesel_savepoint_1")
                    .is_err()
            );
            Ok(())
        })
        .unwrap();

    let kept = kept::table
        .select(kept::id)
        .load::<i32>(connection)
        .unwrap();
    assert_eq!(kept, [1]);
}

#[diesel_test_helper::test]
fn scopes_ended_by_text_controls_report_not_in_transaction() {
    let connection = &mut connection();
    connection
        .transaction::<_, Error, _>(|connection| {
            let inner = connection.transaction::<_, Error, _>(|connection| {
                connection.batch_execute("RELEASE diesel_savepoint_1; SAVEPOINT replacement")?;
                Ok(())
            });
            // `Error`'s equality does not cover `NotInTransaction`.
            assert!(matches!(inner, Err(Error::NotInTransaction)), "{inner:?}");
            assert_eq!(
                depth(connection),
                2,
                "the replacement and its ancestor stay open"
            );
            connection.batch_execute("RELEASE replacement")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(depth(connection), 0);

    for (sql, open_afterwards) in [
        ("COMMIT; BEGIN", 1),
        ("ROLLBACK; BEGIN", 1),
        ("ROLLBACK TO outer_frame", 1),
    ] {
        let connection = &mut self::connection();
        connection.batch_execute("SAVEPOINT outer_frame").unwrap();
        let result = connection.transaction::<_, Error, _>(|connection| {
            connection.batch_execute(sql)?;
            Ok(())
        });
        assert!(
            matches!(result, Err(Error::NotInTransaction)),
            "{sql}: {result:?}"
        );
        assert_eq!(depth(connection), open_afterwards, "{sql}");
        assert!(connection.in_transaction(), "{sql}");
    }
}

#[diesel_test_helper::test]
fn implicit_rollback_of_text_transactions_needs_no_unwinding() {
    for batch in [false, true] {
        let connection = &mut connection();
        connection
            .batch_execute(
                "CREATE TABLE unique_value(value INTEGER UNIQUE ON CONFLICT ROLLBACK);
                 INSERT INTO unique_value VALUES (1);",
            )
            .unwrap();
        execute(connection, batch, "BEGIN").unwrap();
        execute(connection, batch, "SAVEPOINT x").unwrap();
        assert!(execute(connection, batch, "INSERT INTO unique_value VALUES (1)").is_err());
        assert_eq!(depth(connection), 0);
        execute(connection, batch, "INSERT INTO unique_value VALUES (2)").unwrap();
    }
}

#[diesel_test_helper::test]
fn errors_after_a_text_commit_roll_back_nothing() {
    let connection = &mut connection();
    connection
        .batch_execute("CREATE TABLE unique_value(value INTEGER UNIQUE)")
        .unwrap();
    let result = connection.transaction::<_, Error, _>(|connection| {
        connection.batch_execute("COMMIT")?;
        assert!(
            connection
                .batch_execute(
                    "INSERT INTO unique_value VALUES (1); INSERT INTO unique_value VALUES (1)"
                )
                .is_err()
        );
        connection.batch_execute("INSERT INTO unique_value VALUES (2)")?;
        Ok(())
    });
    assert!(matches!(result, Err(Error::NotInTransaction)), "{result:?}");
    assert_eq!(depth(connection), 0);
    connection.batch_execute("SELECT 1").unwrap();
}
