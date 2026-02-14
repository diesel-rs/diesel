#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

/// SQLite resource limits that can be configured per-connection.
///
/// These limits control various aspects of SQLite's behavior and can be used
/// to prevent resource exhaustion attacks or limit query complexity.
///
/// See [SQLite documentation](https://www.sqlite.org/c3ref/limit.html) for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SqliteLimit {
    /// Maximum length of any string or BLOB or table row, in bytes.
    ///
    /// Default: 1,000,000,000 (~1GB)
    Length,

    /// Maximum length of an SQL statement, in bytes.
    ///
    /// Default: 1,000,000,000 (~1GB)
    ///
    /// This can be useful to prevent extremely large SQL statements from being
    /// processed.
    SqlLength,

    /// Maximum number of columns in a table definition, result set, or index,
    /// and also the maximum number of columns in the ORDER BY or GROUP BY
    /// clauses.
    ///
    /// Default: 2,000
    Column,

    /// Maximum depth of the parse tree for any expression.
    ///
    /// Default: 1,000
    ///
    /// This can help prevent stack overflow from deeply nested expressions.
    ExprDepth,

    /// Maximum number of terms in a compound SELECT statement.
    ///
    /// Default: 500
    CompoundSelect,

    /// Maximum number of instructions in a virtual machine program used to
    /// implement an SQL statement.
    ///
    /// Default: 250,000,000
    ///
    /// If [`sqlite3_prepare_v2()`](https://www.sqlite.org/c3ref/prepare.html)
    /// or the equivalent tries to allocate space for more than this many
    /// opcodes in a single prepared statement, an `SQLITE_NOMEM` error is
    /// returned.
    VdbeOp,

    /// Maximum number of arguments on a function.
    ///
    /// Default: 127
    FunctionArg,

    /// Maximum number of attached databases.
    ///
    /// Default: 10
    Attached,

    /// Maximum length of the pattern argument to the
    /// [`LIKE`](https://www.sqlite.org/lang_expr.html#like) or
    /// [`GLOB`](https://www.sqlite.org/lang_expr.html#glob) operators.
    ///
    /// Default: 50,000
    ///
    /// This can help prevent denial-of-service attacks that use very long
    /// LIKE or GLOB patterns.
    LikePatternLength,

    /// Maximum index number of any parameter in an SQL statement.
    ///
    /// Default: 32,766
    VariableNumber,

    /// Maximum recursion depth of triggers.
    ///
    /// Default: 1,000
    TriggerDepth,

    /// Maximum number of auxiliary worker threads that a single prepared
    /// statement may start.
    ///
    /// Default: 0 (varies by SQLite compile-time settings)
    WorkerThreads,
}

impl SqliteLimit {
    /// Convert to the corresponding FFI constant value.
    pub(crate) fn to_ffi(self) -> i32 {
        match self {
            SqliteLimit::Length => ffi::SQLITE_LIMIT_LENGTH,
            SqliteLimit::SqlLength => ffi::SQLITE_LIMIT_SQL_LENGTH,
            SqliteLimit::Column => ffi::SQLITE_LIMIT_COLUMN,
            SqliteLimit::ExprDepth => ffi::SQLITE_LIMIT_EXPR_DEPTH,
            SqliteLimit::CompoundSelect => ffi::SQLITE_LIMIT_COMPOUND_SELECT,
            SqliteLimit::VdbeOp => ffi::SQLITE_LIMIT_VDBE_OP,
            SqliteLimit::FunctionArg => ffi::SQLITE_LIMIT_FUNCTION_ARG,
            SqliteLimit::Attached => ffi::SQLITE_LIMIT_ATTACHED,
            SqliteLimit::LikePatternLength => ffi::SQLITE_LIMIT_LIKE_PATTERN_LENGTH,
            SqliteLimit::VariableNumber => ffi::SQLITE_LIMIT_VARIABLE_NUMBER,
            SqliteLimit::TriggerDepth => ffi::SQLITE_LIMIT_TRIGGER_DEPTH,
            SqliteLimit::WorkerThreads => ffi::SQLITE_LIMIT_WORKER_THREADS,
        }
    }
}
