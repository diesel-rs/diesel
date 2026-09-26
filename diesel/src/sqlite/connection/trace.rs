//! Types for the SQLite trace callback.
//!
//! See [`sqlite3_trace_v2`](https://sqlite.org/c3ref/trace_v2.html)
//! and the [trace event codes](https://sqlite.org/c3ref/c_trace.html).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;

// The `SQLITE_TRACE_*` constants are typed `i32` in old `libsqlite3-sys` and
// `c_uint` in new ones. Normalize them to `u32` here, the single place these
// constants are read. The casts are required for the old versions.
#[allow(clippy::unnecessary_cast)]
pub(crate) const TRACE_STMT: u32 = ffi::SQLITE_TRACE_STMT as u32;
#[allow(clippy::unnecessary_cast)]
pub(crate) const TRACE_PROFILE: u32 = ffi::SQLITE_TRACE_PROFILE as u32;
#[allow(clippy::unnecessary_cast)]
pub(crate) const TRACE_ROW: u32 = ffi::SQLITE_TRACE_ROW as u32;

bitflags::bitflags! {
    /// Trace event mask selecting which events the callback receives.
    ///
    /// Added in SQLite 3.14.0 (2016-08-08). Combine flags with `|`, and
    /// `SqliteTraceFlags::all()` selects every event.
    ///
    /// SQLite's `SQLITE_TRACE_CLOSE` event is intentionally not exposed. Diesel
    /// removes the trace callback before closing the connection, so a close
    /// event can never reach the callback.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SqliteTraceFlags: u32 {
        /// Statement start: fires when a prepared statement begins executing,
        /// delivering the SQL text.
        const STMT = TRACE_STMT;
        /// Statement profile: fires when a statement finishes, delivering the
        /// SQL text and the elapsed time in nanoseconds.
        const PROFILE = TRACE_PROFILE;
        /// Row: fires for every row a query returns. Very frequent and carries
        /// no data, so prefer `STMT` and `PROFILE` for most logging.
        const ROW = TRACE_ROW;
    }
}

/// Trace events delivered to the trace callback.
///
/// The callback receives one of these events based on the mask
/// registered with [`on_trace`](crate::sqlite::SqliteConnection::on_trace).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum SqliteTraceEvent<'a> {
    /// A prepared statement is beginning to execute.
    #[non_exhaustive]
    Statement {
        /// The unexpanded SQL text, with parameter placeholders. For triggers
        /// and nested statements SQLite may report this as a `-- comment`
        /// rather than the SQL itself.
        sql: &'a str,
        /// Whether the statement is read-only, via
        /// [`sqlite3_stmt_readonly`](https://sqlite.org/c3ref/stmt_readonly.html)
        /// (`true` for `SELECT` and read-only `PRAGMA`). Indirect writes, such
        /// as a user-defined function on another connection or a virtual table
        /// with side effects, are not detected.
        readonly: bool,
    },

    /// A prepared statement has finished executing.
    #[non_exhaustive]
    Profile {
        /// The SQL text of the statement (from `sqlite3_sql`).
        sql: &'a str,
        /// Time taken in nanoseconds.
        duration_ns: u64,
        /// Whether the statement is read-only. See
        /// [`SqliteTraceEvent::Statement::readonly`] for details.
        readonly: bool,
    },

    /// A row has been returned from a query.
    ///
    /// Fires for every returned row and carries no data.
    #[non_exhaustive]
    Row,
}

impl SqliteConnection {
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
    use std::sync::atomic::Ordering;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
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
