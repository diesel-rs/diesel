use super::SqliteConnection;

pub(super) use super::CommitDecision;

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
}
