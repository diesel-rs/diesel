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
    /// The callback must not use the database connection. Panics in the
    /// callback abort the process.
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
}
