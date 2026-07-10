use super::SqliteConnection;
use core::num::NonZeroU32;

pub(super) use super::authorizer::{AuthorizerContext, AuthorizerDecision};
pub(super) use super::{BusyDecision, CommitDecision, ProgressDecision};
use super::{SqliteTraceEvent, SqliteTraceFlags};

impl SqliteConnection {
    /// Registers a callback invoked when a transaction is about to be
    /// committed.
    ///
    /// The callback returns a [`CommitDecision`]: `Proceed` lets the commit
    /// complete, `Rollback` converts it into a rollback.
    ///
    /// Only one commit hook can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// The callback runs synchronously as part of the committing
    /// `sqlite3_step()` call, on the thread performing the commit, so it is
    /// never invoked concurrently. Per SQLite, the callback must not use the
    /// connection that triggered it (running any SQL, including a `SELECT`,
    /// counts as use) and is not reentrant. A panic in the callback aborts the
    /// process.
    ///
    /// See: [`sqlite3_commit_hook`](https://www.sqlite.org/c3ref/commit_hook.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, CommitDecision};
    /// use std::sync::{Arc, Mutex};
    ///
    /// diesel::table! {
    ///     users (id) {
    ///         id -> Integer,
    ///         name -> Text,
    ///     }
    /// }
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let commits = Arc::new(Mutex::new(0u32));
    /// let commits2 = commits.clone();
    ///
    /// conn.on_commit(move || {
    ///     *commits2.lock().unwrap() += 1;
    ///     CommitDecision::Proceed
    /// });
    ///
    /// conn.immediate_transaction(|conn| {
    ///     diesel::insert_into(users::table)
    ///         .values(users::name.eq("Alice"))
    ///         .execute(conn)?;
    ///     Ok::<_, diesel::result::Error>(())
    /// }).unwrap();
    ///
    /// assert_eq!(*commits.lock().unwrap(), 1);
    /// ```
    pub fn on_commit<F>(&mut self, hook: F)
    where
        F: FnMut() -> CommitDecision + Send + 'static,
    {
        self.raw_connection.set_commit_hook(hook);
    }

    /// Removes the commit hook. Subsequent commits will not invoke any
    /// callback.
    ///
    /// See [`on_commit`](Self::on_commit) for usage example.
    pub fn remove_commit_hook(&mut self) {
        self.raw_connection.remove_commit_hook();
    }

    /// Registers a callback invoked after a transaction is rolled back.
    ///
    /// This is **not** invoked for the implicit rollback that occurs when
    /// the connection is closed. It **is** invoked when a commit hook forces
    /// a rollback by returning [`CommitDecision::Rollback`].
    ///
    /// Only one rollback hook can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// The callback must not use the database connection. It is invoked
    /// synchronously on the thread driving the connection, so it is never
    /// called concurrently, and like the commit hook it is not reentrant.
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_rollback_hook`](https://www.sqlite.org/c3ref/commit_hook.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicU32, Ordering};
    ///
    /// # use diesel::connection::SimpleConnection;
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// # conn.batch_execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)").unwrap();
    /// let rollbacks = Arc::new(AtomicU32::new(0));
    /// let rb2 = rollbacks.clone();
    ///
    /// conn.on_rollback(move || {
    ///     rb2.fetch_add(1, Ordering::Relaxed);
    /// });
    ///
    /// // Force a rollback by returning an error.
    /// let _ = conn.immediate_transaction(|_conn| {
    ///     Err::<(), _>(diesel::result::Error::RollbackTransaction)
    /// });
    ///
    /// assert_eq!(rollbacks.load(Ordering::Relaxed), 1);
    /// ```
    pub fn on_rollback<F>(&mut self, hook: F)
    where
        F: FnMut() + Send + 'static,
    {
        self.raw_connection.set_rollback_hook(hook);
    }

    /// Removes the rollback hook. Subsequent rollbacks will not invoke any
    /// callback.
    ///
    /// See [`on_rollback`](Self::on_rollback) for usage example.
    pub fn remove_rollback_hook(&mut self) {
        self.raw_connection.remove_rollback_hook();
    }

    /// Registers a progress handler that can interrupt long-running queries.
    ///
    /// The callback is invoked periodically while a query runs. `n` is the
    /// approximate number of virtual-machine instructions between callbacks. It
    /// is a [`NonZeroU32`] so the handler cannot be disabled implicitly by
    /// passing zero. Use [`remove_progress_handler`](Self::remove_progress_handler)
    /// to disable it. Since SQLite 3.41.0 the callback may also fire during
    /// statement preparation.
    ///
    /// The callback returns a [`ProgressDecision`]: `Continue` lets the query
    /// keep executing, `Interrupt` aborts it (causes `SQLITE_INTERRUPT`).
    ///
    /// Only one progress handler can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// The callback must not use the database connection. It is invoked
    /// synchronously on the thread driving the connection, so it is never
    /// called concurrently. Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_progress_handler`](https://www.sqlite.org/c3ref/progress_handler.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, ProgressDecision};
    /// use std::num::NonZeroU32;
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// let cancelled = Arc::new(AtomicBool::new(false));
    /// let cancelled2 = cancelled.clone();
    ///
    /// conn.on_progress(NonZeroU32::new(1000).unwrap(), move || {
    ///     if cancelled2.load(Ordering::Relaxed) {
    ///         ProgressDecision::Interrupt
    ///     } else {
    ///         ProgressDecision::Continue
    ///     }
    /// });
    ///
    /// // Later: remove the handler.
    /// conn.remove_progress_handler();
    /// ```
    pub fn on_progress<F>(&mut self, n: NonZeroU32, hook: F)
    where
        F: FnMut() -> ProgressDecision + Send + 'static,
    {
        self.raw_connection.set_progress_handler(n, hook);
    }

    /// Removes the progress handler. Subsequent queries will not invoke any
    /// callback.
    ///
    /// See [`on_progress`](Self::on_progress) for usage example.
    pub fn remove_progress_handler(&mut self) {
        self.raw_connection.remove_progress_handler();
    }

    /// Registers a callback invoked after each commit of a transaction in
    /// [WAL mode](https://www.sqlite.org/wal.html). It receives a borrowed
    /// connection, the database name (`"main"`, `"temp"`, or an `ATTACH`
    /// alias), and the current WAL page count.
    ///
    /// The hook fires after the commit completes and the write-lock is
    /// released, so the callback may read, write, or
    /// [checkpoint](https://www.sqlite.org/wal.html#ckpt) through `conn`,
    /// provided it leaves no open transaction. A write that commits inside the
    /// callback re-fires the hook re-entrantly, so a callback that writes
    /// unconditionally will recurse until the stack overflows. Guard against
    /// that yourself if the callback writes.
    ///
    /// Only one WAL hook is active at a time, and re-registering replaces it.
    /// `PRAGMA wal_autocheckpoint` installs its own WAL hook and overwrites
    /// this one. A panic in the callback aborts the process.
    ///
    /// See: [`sqlite3_wal_hook`](https://www.sqlite.org/c3ref/wal_hook.html)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use diesel::prelude::*;
    /// # use diesel::connection::SimpleConnection;
    /// # use diesel::sqlite::SqliteConnection;
    /// # let conn = &mut SqliteConnection::establish("test.db").unwrap();
    /// # conn.batch_execute("PRAGMA journal_mode = WAL;").unwrap();
    /// conn.on_wal(|conn, db_name, n_pages| {
    ///     println!("WAL for {db_name}: {n_pages} pages");
    ///     if n_pages > 1000 {
    ///         // The connection may be used here, e.g. to force a checkpoint.
    ///         let _ = conn.batch_execute("PRAGMA wal_checkpoint(TRUNCATE);");
    ///     }
    /// });
    /// ```
    pub fn on_wal<F>(&mut self, hook: F)
    where
        F: Fn(&mut SqliteConnection, &str, u32) + Send + 'static,
    {
        self.raw_connection.set_wal_hook(hook);
    }

    /// Removes the WAL hook. Subsequent commits will not invoke any callback.
    ///
    /// See [`on_wal`](Self::on_wal) for usage example.
    pub fn remove_wal_hook(&mut self) {
        self.raw_connection.remove_wal_hook();
    }

    /// Registers a custom busy handler for lock contention.
    ///
    /// The callback receives the retry count (starting from 0) and returns a
    /// [`BusyDecision`]: `Retry` retries the locked operation, `GiveUp` aborts
    /// and returns `SQLITE_BUSY` to the caller.
    ///
    /// Setting this clears any timeout previously set with
    /// [`set_busy_timeout`](Self::set_busy_timeout). Conversely, calling
    /// `set_busy_timeout` clears this handler. Only one busy handler can be
    /// active at a time per connection.
    ///
    /// The callback must not use the database connection. If the callback
    /// modifies the database, behavior is undefined. SQLite may return
    /// `SQLITE_BUSY` instead of calling the handler to prevent deadlocks.
    ///
    /// The callback is invoked synchronously on the thread driving the
    /// connection, so it is never called concurrently, and per SQLite it is
    /// not reentrant.
    ///
    /// Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_busy_handler`](https://www.sqlite.org/c3ref/busy_handler.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, BusyDecision};
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_busy(|retry_count| {
    ///     if retry_count < 5 {
    ///         thread::sleep(Duration::from_millis(100));
    ///         BusyDecision::Retry
    ///     } else {
    ///         BusyDecision::GiveUp
    ///     }
    /// });
    ///
    /// // Later: remove the handler
    /// conn.remove_busy_handler();
    /// ```
    pub fn on_busy<F>(&mut self, hook: F)
    where
        F: FnMut(i32) -> BusyDecision + Send + 'static,
    {
        self.raw_connection.set_busy_handler(hook);
    }

    /// Removes the custom busy handler.
    ///
    /// See [`on_busy`](Self::on_busy) for usage example.
    pub fn remove_busy_handler(&mut self) {
        self.raw_connection.remove_busy_handler();
    }

    /// Sets a simple timeout-based busy handler.
    ///
    /// When a table is locked, SQLite will sleep and retry until `ms`
    /// milliseconds have elapsed. Pass 0 to disable (return `SQLITE_BUSY`
    /// immediately).
    ///
    /// Setting this clears any custom [`on_busy`](Self::on_busy) handler.
    /// Conversely, calling `on_busy` clears this timeout. For most use cases,
    /// this is simpler than a custom busy handler.
    ///
    /// See: [`sqlite3_busy_timeout`](https://www.sqlite.org/c3ref/busy_timeout.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::prelude::*;
    /// use diesel::sqlite::SqliteConnection;
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// // Wait up to 5 seconds for locked tables
    /// conn.set_busy_timeout(5000);
    /// ```
    pub fn set_busy_timeout(&mut self, ms: i32) {
        self.raw_connection.set_busy_timeout(ms);
    }

    /// Registers an authorizer callback for SQL compilation access control.
    /// Added in SQLite 3.0.0 (June 2004).
    ///
    /// The callback is invoked during `sqlite3_prepare()` (statement
    /// compilation) to control access to database objects. It receives
    /// an [`AuthorizerContext`] describing the operation and returns an
    /// [`AuthorizerDecision`].
    ///
    /// The authorizer is consulted only at statement preparation, so treat it
    /// as defense-in-depth rather than a complete sandbox. SQLite re-checks
    /// already-prepared statements when an authorizer is installed. Removing the
    /// authorizer clears diesel's statement cache so statements prepared while
    /// it was active are re-prepared without it.
    ///
    /// The authorizer may be re-invoked during `sqlite3_step()` if a schema
    /// change triggers statement recompilation.
    ///
    /// The callback must not modify the database connection. It is invoked
    /// synchronously on the thread driving the connection, so it is never
    /// called concurrently.
    ///
    /// Only one authorizer can be active at a time per connection.
    /// Registering a new one replaces the previous. Panics in the callback
    /// abort the process.
    ///
    /// See: [`sqlite3_set_authorizer`](https://sqlite.org/c3ref/set_authorizer.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, AuthorizerContext, AuthorizerDecision};
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_authorize(|ctx| match ctx {
    ///     AuthorizerContext::Delete(_) => AuthorizerDecision::Deny,
    ///     AuthorizerContext::DropTable(_) | AuthorizerContext::DropIndex(_) => {
    ///         AuthorizerDecision::Deny
    ///     }
    ///     _ => AuthorizerDecision::Allow,
    /// });
    ///
    /// // Later: remove the authorizer
    /// conn.remove_authorizer();
    /// ```
    pub fn on_authorize<F>(&mut self, hook: F)
    where
        F: FnMut(AuthorizerContext<'_>) -> AuthorizerDecision + Send + 'static,
    {
        // No cache clear is needed here. Installing a non-null authorizer makes
        // SQLite expire every prepared statement on the connection, and because
        // diesel prepares with `sqlite3_prepare_v3` those statements are
        // transparently re-prepared under the new authorizer on their next step.
        self.raw_connection.set_authorizer(hook);
        // The public documentation doesn't explicitly state what happens with existing
        // prepared statements, so rather be safe and nuke them. We don't
        // expect that people change the authorizer all the time, so this should be fine
        self.statement_cache.clear();
    }

    /// Removes the authorizer callback.
    ///
    /// See [`on_authorize`](Self::on_authorize) for usage example.
    pub fn remove_authorizer(&mut self) {
        self.raw_connection.remove_authorizer();
        // Removing an authorizer does not expire prepared statements: SQLite
        // expires them only when a non-null authorizer is installed. A statement
        // prepared while the authorizer was active can have its decisions
        // compiled in (an `Ignore` on a column read, for example, bakes a `NULL`
        // into the statement), and nothing invalidates that cached statement on
        // removal, so it keeps returning the old result. Clear the cache to force
        // the next query to re-prepare without the authorizer.
        self.statement_cache.clear();
    }

    /// Registers a trace callback for SQL execution monitoring.
    ///
    /// The callback receives the [`SqliteTraceEvent`]s selected by the
    /// [`SqliteTraceFlags`] mask. `ROW` fires once per returned row, so prefer
    /// `STMT`/`PROFILE` for most logging.
    ///
    /// Only one trace callback can be active at a time per connection.
    /// Registering a new one replaces the previous.
    ///
    /// The callback must not use the database connection. It is invoked
    /// synchronously on the thread driving the connection, so it is never
    /// called concurrently. Panics in the callback abort the process.
    ///
    /// See: [`sqlite3_trace_v2`](https://sqlite.org/c3ref/trace_v2.html)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use diesel::prelude::*;
    /// use diesel::sqlite::{SqliteConnection, SqliteTraceFlags, SqliteTraceEvent};
    ///
    /// # let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.on_trace(SqliteTraceFlags::STMT | SqliteTraceFlags::PROFILE, |event| {
    ///     match event {
    ///         SqliteTraceEvent::Statement { sql, readonly, .. } => {
    ///             println!("Executing ({}): {}", if readonly { "read" } else { "write" }, sql);
    ///         }
    ///         SqliteTraceEvent::Profile { sql, duration_ns, .. } => {
    ///             println!("{} took {} ns", sql, duration_ns);
    ///         }
    ///         _ => {}
    ///     }
    /// });
    ///
    /// // Later: remove the trace callback
    /// conn.remove_trace();
    /// ```
    pub fn on_trace<F>(&mut self, mask: SqliteTraceFlags, hook: F)
    where
        F: FnMut(SqliteTraceEvent<'_>) + Send + 'static,
    {
        self.raw_connection.set_trace(mask, hook);
    }

    /// Removes the trace callback.
    ///
    /// See [`on_trace`](Self::on_trace) for usage example.
    pub fn remove_trace(&mut self) {
        self.raw_connection.remove_trace();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::query_dsl::RunQueryDsl;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[derive(crate::QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = crate::sql_types::BigInt)]
        c: i64,
    }

    #[diesel_test_helper::test]
    fn on_commit_fires_on_commit() {
        let conn = &mut connection();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_commit(move || {
            c2.fetch_add(1, Ordering::Relaxed);
            CommitDecision::Proceed
        });

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t1 (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[diesel_test_helper::test]
    fn on_commit_returning_true_forces_rollback() {
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_commit (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        conn.on_commit(|| CommitDecision::Rollback);

        // The transaction will attempt to commit, but the hook will convert
        // it to a rollback. diesel's AnsiTransactionManager will see the
        // failure from the COMMIT statement (sqlite returns an error when
        // the commit hook returns non-zero and the commit is aborted).
        let result = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_commit (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        });

        // The transaction should have been rolled back.
        assert!(result.is_err());

        // Remove the hook so subsequent queries don't fail.
        conn.remove_commit_hook();

        // Verify the row was not persisted.
        let cnt: i64 = crate::sql_query("SELECT COUNT(*) as c FROM t_commit")
            .get_result::<CountResult>(conn)
            .unwrap()
            .c;
        assert_eq!(cnt, 0);
    }

    #[diesel_test_helper::test]
    fn replacing_commit_hook_drops_old() {
        let conn = &mut connection();

        let old_count = Arc::new(AtomicU32::new(0));
        let new_count = Arc::new(AtomicU32::new(0));
        let oc = old_count.clone();
        let nc = new_count.clone();

        conn.on_commit(move || {
            oc.fetch_add(1, Ordering::Relaxed);
            CommitDecision::Proceed
        });

        // Replace with a new hook.
        conn.on_commit(move || {
            nc.fetch_add(1, Ordering::Relaxed);
            CommitDecision::Proceed
        });

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t_replace (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(old_count.load(Ordering::Relaxed), 0);
        assert_eq!(new_count.load(Ordering::Relaxed), 1);
    }

    #[diesel_test_helper::test]
    fn remove_commit_hook_disables_callback() {
        let conn = &mut connection();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_commit(move || {
            c2.fetch_add(1, Ordering::Relaxed);
            CommitDecision::Proceed
        });

        conn.remove_commit_hook();

        conn.immediate_transaction(|conn| {
            crate::sql_query("CREATE TABLE t_rem (id INTEGER PRIMARY KEY)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    #[diesel_test_helper::test]
    fn on_rollback_fires_on_explicit_rollback() {
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rb (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_rollback(move || {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        // Force a rollback by returning Err from the transaction closure.
        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rb (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Err::<(), _>(crate::result::Error::RollbackTransaction)
        });

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[diesel_test_helper::test]
    fn on_rollback_fires_when_commit_hook_forces_rollback() {
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rb2 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let rb_count = Arc::new(AtomicU32::new(0));
        let rb2 = rb_count.clone();

        conn.on_commit(|| CommitDecision::Rollback);
        conn.on_rollback(move || {
            rb2.fetch_add(1, Ordering::Relaxed);
        });

        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rb2 (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Ok::<_, crate::result::Error>(())
        });

        // Rollback hook should have fired.
        assert_eq!(rb_count.load(Ordering::Relaxed), 1);

        conn.remove_commit_hook();
        conn.remove_rollback_hook();

        // Verify the row was not persisted.
        let cnt: i64 = crate::sql_query("SELECT COUNT(*) as c FROM t_rb2")
            .get_result::<CountResult>(conn)
            .unwrap()
            .c;
        assert_eq!(cnt, 0);
    }

    #[diesel_test_helper::test]
    fn on_rollback_does_not_fire_on_connection_close() {
        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        {
            let conn = &mut connection();
            conn.on_rollback(move || {
                c2.fetch_add(1, Ordering::Relaxed);
            });
            // conn is dropped here: implicit close, not a rollback.
        }

        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    #[diesel_test_helper::test]
    fn remove_rollback_hook_disables_callback() {
        let conn = &mut connection();

        crate::sql_query("CREATE TABLE t_rem_rb (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_rollback(move || {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        conn.remove_rollback_hook();

        let _ = conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_rem_rb (id) VALUES (1)")
                .execute(conn)
                .unwrap();
            Err::<(), _>(crate::result::Error::RollbackTransaction)
        });

        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    // A recursive CTE heavy enough that the progress handler fires while it runs.
    const HEAVY_QUERY: &str = "WITH RECURSIVE c(x) AS \
        (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 100000) SELECT count(*) FROM c";

    #[diesel_test_helper::test]
    fn on_progress_interrupts_query() {
        let conn = &mut connection();

        conn.on_progress(NonZeroU32::new(1).unwrap(), || ProgressDecision::Interrupt);

        let result = crate::sql_query(HEAVY_QUERY).execute(conn);
        assert!(
            result.is_err(),
            "the query should be interrupted by the progress handler"
        );
    }

    #[diesel_test_helper::test]
    fn remove_progress_handler_stops_interruption() {
        let conn = &mut connection();

        conn.on_progress(NonZeroU32::new(1).unwrap(), || ProgressDecision::Interrupt);
        conn.remove_progress_handler();

        // With the handler removed the same query runs to completion.
        let result = crate::sql_query(HEAVY_QUERY).execute(conn);
        assert!(
            result.is_ok(),
            "the query should complete after the handler is removed"
        );
    }

    // WAL hook tests
    //
    // Gated out on WASM because these tests need a file-backed database
    // (WAL mode does not work with `:memory:`), and `tempfile::tempdir()`
    // panics on WASM due to the lack of a filesystem.
    //
    // The WAL API itself (`on_wal`, `remove_wal_hook`) is available on all
    // platforms, including WASM. The file-backed databases used here cannot
    // be created on WASM, so only the tests are gated out.

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    /// Helper: create a file-backed connection in WAL mode (WAL requires a real
    /// file, and every WAL test below wants the connection already in WAL mode).
    fn wal_connection() -> (SqliteConnection, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        crate::sql_query("PRAGMA journal_mode=WAL")
            .execute(&mut conn)
            .unwrap();
        (conn, dir)
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_wal_fires_in_wal_mode() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let events: Arc<std::sync::Mutex<Vec<(String, u32)>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let events2 = events.clone();

        conn.on_wal(move |_, db_name, n_pages| {
            events2.lock().unwrap().push((db_name.to_owned(), n_pages));
        });

        crate::sql_query("INSERT INTO t_wal (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        let events = events.lock().unwrap();
        assert!(
            !events.is_empty(),
            "WAL hook should have fired at least once"
        );
        assert!(
            events.iter().all(|(db_name, _)| db_name == "main"),
            "db_name should always be \"main\""
        );
        assert!(
            events.iter().any(|(_, n_pages)| *n_pages > 0),
            "n_pages should be positive"
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn replacing_wal_hook_drops_old() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal2 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let old_count = Arc::new(AtomicU32::new(0));
        let new_count = Arc::new(AtomicU32::new(0));

        let c_old = old_count.clone();
        conn.on_wal(move |_, _, _| {
            c_old.fetch_add(1, Ordering::Relaxed);
        });

        // Replace with a new hook.
        let c_new = new_count.clone();
        conn.on_wal(move |_, _, _| {
            c_new.fetch_add(1, Ordering::Relaxed);
        });

        crate::sql_query("INSERT INTO t_wal2 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        // Old hook should NOT have fired after replacement.
        let old_before = old_count.load(Ordering::Relaxed);
        crate::sql_query("INSERT INTO t_wal2 (id) VALUES (2)")
            .execute(conn)
            .unwrap();
        assert_eq!(
            old_count.load(Ordering::Relaxed),
            old_before,
            "old WAL hook should not fire after replacement"
        );
        assert!(
            new_count.load(Ordering::Relaxed) > 0,
            "new WAL hook should fire"
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn remove_wal_hook_disables_callback() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal3 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_wal(move |_, _, _| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        conn.remove_wal_hook();

        crate::sql_query("INSERT INTO t_wal3 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_hook_does_not_fire_in_default_journal_mode() {
        // A plain file connection left in the default journal mode ("delete"),
        // not WAL, so `wal_connection()` (which enables WAL) is not used here.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = &mut SqliteConnection::establish(path.to_str().unwrap()).unwrap();

        crate::sql_query("CREATE TABLE t_wal4 (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_wal(move |_, _, _| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        crate::sql_query("INSERT INTO t_wal4 (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        assert_eq!(
            count.load(Ordering::Relaxed),
            0,
            "WAL hook should not fire when not in WAL mode"
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_wal_can_use_borrowed_connection() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal_use (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let counts: Arc<std::sync::Mutex<Vec<i64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
        let counts2 = counts.clone();

        conn.on_wal(move |conn, _db_name, _n_pages| {
            // Read through the borrowed connection, a fresh implicit read
            // transaction that finalizes on return.
            let c = crate::sql_query("SELECT COUNT(*) AS c FROM t_wal_use")
                .get_result::<CountResult>(conn)
                .unwrap()
                .c;
            counts2.lock().unwrap().push(c);
        });

        crate::sql_query("INSERT INTO t_wal_use (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        let observed = counts.lock().unwrap();
        assert!(!observed.is_empty(), "WAL hook should have fired");
        assert!(
            observed.contains(&1),
            "callback should observe the committed row through the connection"
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_wal_callback_write_re_enters_hook() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal_re (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE t_wal_log (id INTEGER PRIMARY KEY AUTOINCREMENT)")
            .execute(conn)
            .unwrap();

        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        conn.on_wal(move |conn, _db_name, _n_pages| {
            let n = calls2.fetch_add(1, Ordering::Relaxed);
            // Write on the first invocation only. The write commits in WAL mode
            // and re-fires the hook re-entrantly. Gating on `n == 0` bounds the
            // recursion to a single nested call instead of overflowing the stack.
            if n == 0 {
                crate::sql_query("INSERT INTO t_wal_log DEFAULT VALUES")
                    .execute(conn)
                    .unwrap();
            }
        });

        crate::sql_query("INSERT INTO t_wal_re (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        // The outer commit fired the hook, and the write inside it re-fired the
        // hook once more, so the `Fn` callback ran twice.
        assert_eq!(
            calls.load(Ordering::Relaxed),
            2,
            "a committing write inside the callback re-enters the hook"
        );

        // The write performed inside the callback took effect.
        let logged: i64 = crate::sql_query("SELECT COUNT(*) AS c FROM t_wal_log")
            .get_result::<CountResult>(conn)
            .unwrap()
            .c;
        assert_eq!(
            logged, 1,
            "the write performed inside the callback should persist"
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_wal_fires_once_per_transaction_commit() {
        let (conn, _dir) = &mut wal_connection();

        crate::sql_query("CREATE TABLE t_wal_txn (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let count = Arc::new(AtomicU32::new(0));
        let c2 = count.clone();

        conn.on_wal(move |_, _, _| {
            c2.fetch_add(1, Ordering::Relaxed);
        });

        // Several writes inside one explicit transaction commit together, so the
        // WAL hook fires once for the single commit, not once per statement.
        conn.immediate_transaction(|conn| {
            crate::sql_query("INSERT INTO t_wal_txn (id) VALUES (1)").execute(conn)?;
            crate::sql_query("INSERT INTO t_wal_txn (id) VALUES (2)").execute(conn)?;
            crate::sql_query("INSERT INTO t_wal_txn (id) VALUES (3)").execute(conn)?;
            Ok::<_, crate::result::Error>(())
        })
        .unwrap();

        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "the WAL hook should fire once per transaction commit, not per statement"
        );
    }

    // Busy handler test.
    //
    // Gated out on WASM: it needs two connections to a shared file-backed
    // database (`:memory:` connections do not share a lock), and
    // `tempfile::tempdir()` panics on WASM due to the lack of a filesystem.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn on_busy_handler_is_invoked_on_lock_contention() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("busy.db");
        let url = path.to_str().unwrap();

        // One connection acquires and holds a write lock.
        let mut holder = SqliteConnection::establish(url).unwrap();
        crate::sql_query("CREATE TABLE t_busy (id INTEGER PRIMARY KEY)")
            .execute(&mut holder)
            .unwrap();
        crate::sql_query("BEGIN IMMEDIATE")
            .execute(&mut holder)
            .unwrap();

        // A second connection registers a busy handler that records the call
        // and gives up.
        let mut contender = SqliteConnection::establish(url).unwrap();
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();
        contender.on_busy(move |_retry_count| {
            calls2.fetch_add(1, Ordering::Relaxed);
            BusyDecision::GiveUp
        });

        // The write contends with the held lock, so the busy handler fires.
        // Because it gives up, the write fails instead of blocking.
        let result = crate::sql_query("INSERT INTO t_busy (id) VALUES (1)").execute(&mut contender);

        assert!(
            result.is_err(),
            "the contended write should fail once the busy handler gives up"
        );
        assert!(
            calls.load(Ordering::Relaxed) >= 1,
            "the busy handler should have been invoked at least once"
        );
    }

    #[diesel_test_helper::test]
    fn on_authorize_deny_rejects_statement() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE auth_basic (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();

        conn.on_authorize(move |_ctx| {
            calls2.fetch_add(1, Ordering::Relaxed);
            AuthorizerDecision::Deny
        });

        // The authorizer is consulted while the statement is prepared, denies
        // it, and the statement is rejected.
        let denied = crate::sql_query("SELECT id FROM auth_basic").execute(conn);
        assert!(denied.is_err(), "a denied statement should fail to prepare");
        assert!(
            calls.load(Ordering::Relaxed) > 0,
            "the authorizer callback should have been invoked"
        );

        // Removing the authorizer restores access.
        conn.remove_authorizer();
        crate::sql_query("SELECT id FROM auth_basic")
            .execute(conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn remove_authorizer_re_prepares_cached_statements() {
        use crate::prelude::*;
        use crate::sqlite::AuthorizerContext;

        crate::table! {
            auth_ignore_items (id) {
                id -> Integer,
            }
        }

        let conn = &mut connection();
        crate::sql_query("CREATE TABLE auth_ignore_items (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO auth_ignore_items (id) VALUES (42)")
            .execute(conn)
            .unwrap();

        // An authorizer that ignores column reads. SQLite bakes a NULL
        // substitution for the column into the prepared (and cached) statement.
        // A typed query is used because raw `sql_query` is never cached.
        conn.on_authorize(|ctx| match ctx {
            AuthorizerContext::Read(_) => AuthorizerDecision::Ignore,
            _ => AuthorizerDecision::Allow,
        });
        let ignored = auth_ignore_items::table
            .select(auth_ignore_items::id.nullable())
            .load::<Option<i32>>(conn)
            .unwrap();
        assert_eq!(
            ignored,
            vec![None],
            "Ignore substitutes NULL for the column"
        );

        // SQLite does not expire prepared statements when an authorizer is
        // removed, so `remove_authorizer` clears diesel's statement cache to
        // force the cached statement (with its baked-in NULL) to be re-prepared.
        conn.remove_authorizer();
        let restored = auth_ignore_items::table
            .select(auth_ignore_items::id.nullable())
            .load::<Option<i32>>(conn)
            .unwrap();
        assert_eq!(
            restored,
            vec![Some(42)],
            "after removing the authorizer the real value is returned"
        );
    }

    #[diesel_test_helper::test]
    fn on_authorize_re_prepares_cached_statements() {
        use crate::prelude::*;
        use crate::sqlite::AuthorizerContext;

        crate::table! {
            auth_replace_items (id) {
                id -> Integer,
            }
        }

        let conn = &mut connection();
        crate::sql_query("CREATE TABLE auth_replace_items (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO auth_replace_items (id) VALUES (42)")
            .execute(conn)
            .unwrap();

        // A first authorizer that allows everything. The typed query is
        // prepared and cached while it is active, returning the real value.
        conn.on_authorize(|_ctx| AuthorizerDecision::Allow);
        let allowed = auth_replace_items::table
            .select(auth_replace_items::id.nullable())
            .load::<Option<i32>>(conn)
            .unwrap();
        assert_eq!(
            allowed,
            vec![Some(42)],
            "the allow-all authorizer returns the real value"
        );

        // Installing the replacement authorizer expires the cached statement
        // (SQLite expires all prepared statements when a non-null authorizer is
        // set), so `sqlite3_prepare_v3` re-prepares it under the new authorizer
        // and it now yields NULL. This documents that no diesel-side cache clear
        // is required on the install path.
        conn.on_authorize(|ctx| match ctx {
            AuthorizerContext::Read(_) => AuthorizerDecision::Ignore,
            _ => AuthorizerDecision::Allow,
        });
        let ignored = auth_replace_items::table
            .select(auth_replace_items::id.nullable())
            .load::<Option<i32>>(conn)
            .unwrap();
        assert_eq!(
            ignored,
            vec![None],
            "after replacing the authorizer the new decision takes effect"
        );
    }

    #[diesel_test_helper::test]
    fn on_trace_reports_statement_and_profile() {
        use std::sync::Mutex;

        let conn = &mut connection();
        crate::sql_query("CREATE TABLE t_trace (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        // (sql, readonly) for Statement events, and the SQL of Profile events.
        let stmts: Arc<Mutex<Vec<(String, bool)>>> = Arc::new(Mutex::new(Vec::new()));
        let profiled: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let stmts2 = stmts.clone();
        let profiled2 = profiled.clone();

        conn.on_trace(
            SqliteTraceFlags::STMT | SqliteTraceFlags::PROFILE,
            move |event| match event {
                SqliteTraceEvent::Statement { sql, readonly } => {
                    stmts2.lock().unwrap().push((sql.to_owned(), readonly));
                }
                SqliteTraceEvent::Profile { sql, .. } => {
                    profiled2.lock().unwrap().push(sql.to_owned());
                }
                _ => {}
            },
        );

        crate::sql_query("SELECT id FROM t_trace")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO t_trace (id) VALUES (1)")
            .execute(conn)
            .unwrap();

        let stmts = stmts.lock().unwrap();
        assert!(
            stmts
                .iter()
                .any(|(sql, ro)| sql.contains("SELECT id FROM t_trace") && *ro),
            "the SELECT should be traced and reported read-only"
        );
        assert!(
            stmts
                .iter()
                .any(|(sql, ro)| sql.contains("INSERT INTO t_trace") && !*ro),
            "the INSERT should be traced and reported not read-only"
        );
        assert!(
            !profiled.lock().unwrap().is_empty(),
            "at least one Profile event should have fired"
        );
    }

    #[diesel_test_helper::test]
    fn remove_trace_stops_events() {
        use std::sync::atomic::AtomicUsize;

        let conn = &mut connection();
        let count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let count2 = count.clone();

        conn.on_trace(SqliteTraceFlags::STMT, move |_event| {
            count2.fetch_add(1, Ordering::Relaxed);
        });
        crate::sql_query("SELECT 1").execute(conn).unwrap();
        let after_first = count.load(Ordering::Relaxed);
        assert!(after_first > 0, "trace should fire while registered");

        conn.remove_trace();
        crate::sql_query("SELECT 1").execute(conn).unwrap();
        assert_eq!(
            count.load(Ordering::Relaxed),
            after_first,
            "no trace events should fire after remove_trace"
        );
    }
}
