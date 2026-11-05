use super::raw::RawConnection;
use super::{SqliteConnection, StrQueryHelper};
use crate::connection::{InstrumentationEvent, TransactionManagerStatus};
use crate::result::QueryResult;
use core::ffi::CStr;

#[derive(Clone, Copy)]
pub(super) enum QuerySensitivity {
    Public,
    Sensitive,
}

impl SqliteConnection {
    /// Runs uncached SQL to completion, reporting it to instrumentation and in error messages only as `[REDACTED]` while keeping authorization and error kinds.
    ///
    /// [`on_trace`](Self::on_trace) and [`on_authorize`](Self::on_authorize) callbacks still see the SQL, and erasing `query` afterwards is up to its owner.
    ///
    /// # Errors
    ///
    /// Returns an error if SQLite rejects execution.
    ///
    /// # Example
    ///
    /// ```
    /// # use diesel::Connection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # fn main() -> diesel::QueryResult<()> {
    /// let mut connection = SqliteConnection::establish(":memory:").unwrap();
    /// connection.batch_execute_sensitive(c"PRAGMA key = 'example'")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn batch_execute_sensitive(&mut self, query: &CStr) -> QueryResult<()> {
        self.batch_execute_with_diagnostics("[REDACTED]", |connection| {
            connection.exec_sensitive(query)
        })
    }

    pub(super) fn batch_execute_with_diagnostics(
        &mut self,
        diagnostic_query: &str,
        execute: impl FnOnce(&RawConnection) -> QueryResult<()>,
    ) -> QueryResult<()> {
        let diagnostic_query = StrQueryHelper::new(diagnostic_query);
        self.instrumentation
            .on_connection_event(InstrumentationEvent::StartQuery {
                query: &diagnostic_query,
            });
        let result = execute(&self.raw_connection);
        self.instrumentation
            .on_connection_event(InstrumentationEvent::FinishQuery {
                query: &diagnostic_query,
                error: result.as_ref().err(),
            });
        // SQLite ends the transaction on some failures, e.g. an aborting commit hook.
        if result.is_err() && self.raw_connection.is_autocommit() {
            self.transaction_state.status = TransactionManagerStatus::Valid(Default::default());
        }
        result
    }
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::super::SqliteConnection;
    use crate::connection::{Connection, InstrumentationEvent, SimpleConnection};
    use crate::prelude::*;
    use crate::result::{DatabaseErrorKind, Error};
    use crate::sqlite::{
        AuthorizerContext, AuthorizerDecision, CommitDecision, SqliteTraceEvent, SqliteTraceFlags,
    };
    use std::sync::{Arc, Mutex};

    const KEY: &str = "synthetic_key_7c91a2";

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    fn observe(connection: &mut SqliteConnection) -> Arc<Mutex<Vec<(bool, bool)>>> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let records = Arc::clone(&events);
        connection.set_instrumentation(move |event: InstrumentationEvent<'_>| {
            assert!(!format!("{event:?}").contains(KEY));
            match event {
                InstrumentationEvent::StartQuery { query, .. } => {
                    assert!(!format!("{query:?}\n{query}").contains(KEY));
                    records.lock().unwrap().push((true, false));
                }
                InstrumentationEvent::FinishQuery { query, error, .. } => {
                    assert!(!format!("{query:?}\n{query}").contains(KEY));
                    if let Some(error) = error {
                        assert!(!format!("{error:?}\n{error}").contains(KEY));
                    }
                    records.lock().unwrap().push((false, error.is_some()));
                }
                InstrumentationEvent::CacheQuery { sql, .. } => assert!(!sql.contains(KEY)),
                _ => {}
            }
        });
        events
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_redacts_success_and_failure_diagnostics() {
        let mut connection = connection();
        let events = observe(&mut connection);
        connection
            .batch_execute_sensitive(c"PRAGMA key = 'synthetic_key_7c91a2'")
            .unwrap();
        let error = connection
            .batch_execute_sensitive(c"SELECT synthetic_key_7c91a2")
            .unwrap_err();
        assert!(matches!(
            error,
            Error::DatabaseError(DatabaseErrorKind::Unknown, _)
        ));
        assert!(!format!("{error:?}\n{error}").contains(KEY));
        assert_eq!(
            *events.lock().unwrap(),
            [(true, false), (false, false), (true, false), (false, true)]
        );
    }

    #[cfg(all(
        feature = "std",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[test]
    fn sensitive_callback_error_does_not_reach_panic_hook() {
        let stderr = super::super::oom_test_support::run_in_child(|| {
            let error = connection()
                .batch_execute_sensitive(c"SELECT diesel_manage_updated_at('synthetic_key_7c91a2')")
                .unwrap_err();
            assert!(matches!(error, Error::DatabaseError(..)));
            assert!(!format!("{error:?}\n{error}").contains(KEY));
        });
        assert!(stderr.is_empty(), "{stderr:?}");
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_preserves_authorization() {
        let mut connection = connection();
        let decisions = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&decisions);
        connection.on_authorize(move |context| match context {
            AuthorizerContext::Pragma(pragma) if pragma.name == Some("key") => {
                assert_eq!(pragma.argument, Some(KEY));
                observed.lock().unwrap().push(AuthorizerDecision::Deny);
                AuthorizerDecision::Deny
            }
            _ => AuthorizerDecision::Allow,
        });
        let events = observe(&mut connection);
        for _ in 0..2 {
            let error = connection
                .batch_execute_sensitive(c"PRAGMA key = 'synthetic_key_7c91a2'")
                .unwrap_err();
            assert!(matches!(
                error,
                Error::DatabaseError(DatabaseErrorKind::Unknown, _)
            ));
        }
        assert_eq!(decisions.lock().unwrap().len(), 2);
        assert_eq!(
            *events.lock().unwrap(),
            [(true, false), (false, true), (true, false), (false, true)]
        );
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_preserves_constraint_error_categories() {
        let mut connection = connection();
        connection
            .batch_execute(&format!(
                "CREATE TABLE {KEY} (id INTEGER PRIMARY KEY, v INTEGER NOT NULL CHECK (v > 0))"
            ))
            .unwrap();
        let events = observe(&mut connection);
        connection
            .batch_execute_sensitive(c"INSERT INTO synthetic_key_7c91a2 VALUES (1, 1)")
            .unwrap();
        for (sql, expected) in [
            (
                c"INSERT INTO synthetic_key_7c91a2 VALUES (1, 2)",
                DatabaseErrorKind::UniqueViolation,
            ),
            (
                c"INSERT INTO synthetic_key_7c91a2 VALUES (2, NULL)",
                DatabaseErrorKind::NotNullViolation,
            ),
            (
                c"INSERT INTO synthetic_key_7c91a2 VALUES (2, -1)",
                DatabaseErrorKind::CheckViolation,
            ),
        ] {
            let Error::DatabaseError(kind, info) =
                connection.batch_execute_sensitive(sql).unwrap_err()
            else {
                panic!("expected a database error");
            };
            assert_eq!(kind, expected);
            assert!(!info.message().contains(KEY));
        }
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| **event == (false, true))
                .count(),
            3
        );
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_preserves_foreign_key_error_categories() {
        let mut connection = connection();
        connection
            .batch_execute(
                "PRAGMA foreign_keys = ON;
             CREATE TABLE parent (id INTEGER PRIMARY KEY);
             CREATE TABLE child (parent_id INTEGER REFERENCES parent(id) ON DELETE RESTRICT);
             INSERT INTO parent VALUES (1);
             INSERT INTO child VALUES (1)",
            )
            .unwrap();
        observe(&mut connection);
        for query in [c"DELETE FROM parent", c"INSERT INTO child VALUES (2)"] {
            let error = connection.batch_execute_sensitive(query).unwrap_err();
            assert!(matches!(
                error,
                Error::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _)
            ));
        }
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_keeps_raw_tracing_privileged_and_finishes_statements() {
        let mut connection = connection();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let records = Arc::clone(&trace);
        connection.on_trace(
            SqliteTraceFlags::STMT | SqliteTraceFlags::PROFILE,
            move |event| {
                if let SqliteTraceEvent::Statement { sql, .. }
                | SqliteTraceEvent::Profile { sql, .. } = event
                {
                    records.lock().unwrap().push(sql.to_owned());
                }
            },
        );
        let events = observe(&mut connection);
        connection.batch_execute_sensitive(c"PRAGMA key = 'synthetic_key_7c91a2'; CREATE TABLE complete (id INTEGER); INSERT INTO complete VALUES (1) RETURNING id; INSERT INTO complete VALUES (2)").unwrap();
        assert!(trace.lock().unwrap().iter().any(|sql| sql.contains(KEY)));
        assert_eq!(*events.lock().unwrap(), [(true, false), (false, false)]);
        assert_no_statements(&mut connection);
        crate::table! { complete (id) { id -> Integer, } }
        assert_eq!(
            complete::table
                .select(complete::id)
                .load::<i32>(&mut connection)
                .unwrap(),
            [1, 2]
        );
    }

    #[expect(
        unsafe_code,
        reason = "SQLite statement enumeration only reads the borrowed connection"
    )]
    fn assert_no_statements(connection: &mut SqliteConnection) {
        // SAFETY: The callback only enumerates statements on the live borrowed connection.
        unsafe {
            connection.with_raw_connection(|raw| {
                assert!(super::super::ffi::sqlite3_next_stmt(raw, core::ptr::null_mut()).is_null());
            });
        }
    }

    #[diesel_test_helper::test]
    fn sensitive_execution_reconciles_an_aborted_transaction() {
        let mut connection = connection();
        connection
            .batch_execute("CREATE TABLE rolled_back (id INTEGER)")
            .unwrap();
        connection.on_commit(|| CommitDecision::Rollback);
        connection.begin_test_transaction().unwrap();
        observe(&mut connection);
        let error = connection
            .batch_execute_sensitive(
                c"INSERT INTO rolled_back VALUES (1); COMMIT; PRAGMA key = 'synthetic_key_7c91a2'",
            )
            .unwrap_err();
        assert!(matches!(error, Error::DatabaseError(_, _)));
        connection.remove_commit_hook();
        connection.transaction::<_, Error, _>(|_| Ok(())).unwrap();
        crate::table! { rolled_back (id) { id -> Integer, } }
        assert_eq!(
            rolled_back::table
                .count()
                .get_result::<i64>(&mut connection)
                .unwrap(),
            0
        );
    }
}
