//! Types for the SQLite trace callback.
//!
//! See [`sqlite3_trace_v2`](https://sqlite.org/c3ref/trace_v2.html)
//! and the [trace event codes](https://sqlite.org/c3ref/c_trace.html).

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

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
