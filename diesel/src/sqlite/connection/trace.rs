//! Types for the SQLite trace callback.
//!
//! See [`sqlite3_trace_v2`](https://sqlite.org/c3ref/trace_v2.html)
//! and the [trace event codes](https://sqlite.org/c3ref/c_trace.html).

use core::ops::BitOr;

/// Trace event mask (bitmask) for selecting which events to receive.
///
/// Added in SQLite 3.14.0 (2016-08-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SqliteTraceFlags(u32);

impl SqliteTraceFlags {
    /// Statement start event.
    ///
    /// Fires when a prepared statement first begins executing.
    /// The callback receives the SQL text.
    pub const STMT: Self = Self(0x01);

    /// Statement profiling event.
    ///
    /// Fires when a prepared statement finishes executing.
    /// The callback receives the SQL text and elapsed time in nanoseconds.
    pub const PROFILE: Self = Self(0x02);

    /// Row event.
    ///
    /// **Performance Warning**: Fires for EVERY row returned by a query.
    /// This can be extremely frequent for large result sets and may
    /// significantly impact performance. Prefer `STMT` and `PROFILE`
    /// for most logging use cases.
    pub const ROW: Self = Self(0x04);

    /// Connection close event.
    ///
    /// Fires when the database connection closes.
    pub const CLOSE: Self = Self(0x08);

    /// All events.
    ///
    /// Use with caution: includes `ROW` which is very frequent.
    pub const ALL: Self = Self(0x0F);

    /// Returns the raw bitmask value.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Creates a new mask from raw bits.
    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns true if this mask contains the given flag.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for SqliteTraceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Trace events delivered to the trace callback.
///
/// The callback receives one of these events based on the mask
/// registered with [`on_trace`](crate::sqlite::SqliteConnection::on_trace).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SqliteTraceEvent<'a> {
    /// A prepared statement is beginning to execute.
    ///
    /// Contains the unexpanded SQL text (with parameter placeholders).
    Statement {
        /// The SQL text of the statement.
        sql: &'a str,
    },

    /// A prepared statement has finished executing.
    ///
    /// Contains the SQL text and elapsed time.
    Profile {
        /// The SQL text of the statement.
        sql: &'a str,
        /// Time taken in nanoseconds.
        duration_ns: u64,
    },

    /// A row has been returned from a query.
    ///
    /// **Note**: This fires for every row and does not include row data.
    Row,

    /// The database connection is closing.
    Close,
}
