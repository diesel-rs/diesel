#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

/// SQLite resource limits that can be configured per-connection.
///
/// These control aspects of SQLite's behavior and can be used to prevent
/// resource exhaustion or limit query complexity.
///
/// Each variant exposes two associated constants: `DEFAULT_*_LIMIT` (SQLite's
/// documented default) and `SAFE_*_LIMIT` (the hardened value applied by
/// [`SqliteConnection::set_recommended_security_limits`](crate::sqlite::SqliteConnection::set_recommended_security_limits)).
/// A connection's actual runtime default can differ from `DEFAULT_*_LIMIT`
/// because some builds raise the compile-time maximum (for example the bundled
/// `libsqlite3-sys` raises `FunctionArg` and `VariableNumber`).
///
/// See the [SQLite documentation](https://www.sqlite.org/c3ref/limit.html) for details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SqliteLimit {
    /// Maximum length of any string or BLOB or table row, in bytes.
    ///
    /// See [`DEFAULT_LENGTH_LIMIT`](Self::DEFAULT_LENGTH_LIMIT) and
    /// [`SAFE_LENGTH_LIMIT`](Self::SAFE_LENGTH_LIMIT).
    Length,

    /// Maximum length of an SQL statement, in bytes.
    ///
    /// See [`DEFAULT_SQL_LENGTH_LIMIT`](Self::DEFAULT_SQL_LENGTH_LIMIT) and
    /// [`SAFE_SQL_LENGTH_LIMIT`](Self::SAFE_SQL_LENGTH_LIMIT).
    SqlLength,

    /// Maximum number of columns in a table definition, result set, or index,
    /// and also the maximum number of columns in the ORDER BY or GROUP BY
    /// clauses.
    ///
    /// See [`DEFAULT_COLUMN_COUNT_LIMIT`](Self::DEFAULT_COLUMN_COUNT_LIMIT) and
    /// [`SAFE_COLUMN_COUNT_LIMIT`](Self::SAFE_COLUMN_COUNT_LIMIT).
    ColumnCount,

    /// Maximum depth of the parse tree for any expression.
    ///
    /// This can help prevent stack overflow from deeply nested expressions.
    ///
    /// See [`DEFAULT_EXPR_DEPTH_LIMIT`](Self::DEFAULT_EXPR_DEPTH_LIMIT) and
    /// [`SAFE_EXPR_DEPTH_LIMIT`](Self::SAFE_EXPR_DEPTH_LIMIT).
    ExprDepth,

    /// Maximum number of terms in a compound SELECT statement.
    ///
    /// See [`DEFAULT_COMPOUND_SELECT_LIMIT`](Self::DEFAULT_COMPOUND_SELECT_LIMIT)
    /// and [`SAFE_COMPOUND_SELECT_LIMIT`](Self::SAFE_COMPOUND_SELECT_LIMIT).
    CompoundSelect,

    /// Maximum number of instructions in a virtual machine program used to
    /// implement an SQL statement.
    ///
    /// If [`sqlite3_prepare_v2()`](https://www.sqlite.org/c3ref/prepare.html)
    /// or the equivalent tries to allocate space for more than this many
    /// opcodes in a single prepared statement, an `SQLITE_NOMEM` error is
    /// returned.
    ///
    /// See [`DEFAULT_VDBE_OP_LIMIT`](Self::DEFAULT_VDBE_OP_LIMIT) and
    /// [`SAFE_VDBE_OP_LIMIT`](Self::SAFE_VDBE_OP_LIMIT).
    VdbeOp,

    /// Maximum number of arguments on a function.
    ///
    /// See [`DEFAULT_FUNCTION_ARG_LIMIT`](Self::DEFAULT_FUNCTION_ARG_LIMIT) and
    /// [`SAFE_FUNCTION_ARG_LIMIT`](Self::SAFE_FUNCTION_ARG_LIMIT).
    FunctionArg,

    /// Maximum number of attached databases.
    ///
    /// See [`DEFAULT_ATTACHED_LIMIT`](Self::DEFAULT_ATTACHED_LIMIT) and
    /// [`SAFE_ATTACHED_LIMIT`](Self::SAFE_ATTACHED_LIMIT).
    Attached,

    /// Maximum length of the pattern argument to the
    /// [`LIKE`](https://www.sqlite.org/lang_expr.html#like) or
    /// [`GLOB`](https://www.sqlite.org/lang_expr.html#glob) operators.
    ///
    /// See [`DEFAULT_LIKE_PATTERN_LENGTH_LIMIT`](Self::DEFAULT_LIKE_PATTERN_LENGTH_LIMIT)
    /// and [`SAFE_LIKE_PATTERN_LENGTH_LIMIT`](Self::SAFE_LIKE_PATTERN_LENGTH_LIMIT).
    LikePatternLength,

    /// Maximum index number of any parameter in an SQL statement.
    ///
    /// See [`DEFAULT_VARIABLE_NUMBER_LIMIT`](Self::DEFAULT_VARIABLE_NUMBER_LIMIT)
    /// and [`SAFE_VARIABLE_NUMBER_LIMIT`](Self::SAFE_VARIABLE_NUMBER_LIMIT).
    VariableNumber,

    /// Maximum recursion depth of triggers.
    ///
    /// See [`DEFAULT_TRIGGER_DEPTH_LIMIT`](Self::DEFAULT_TRIGGER_DEPTH_LIMIT) and
    /// [`SAFE_TRIGGER_DEPTH_LIMIT`](Self::SAFE_TRIGGER_DEPTH_LIMIT).
    TriggerDepth,

    /// Maximum number of auxiliary worker threads that a single prepared
    /// statement may start.
    ///
    /// See [`DEFAULT_WORKER_THREADS_LIMIT`](Self::DEFAULT_WORKER_THREADS_LIMIT)
    /// and [`SAFE_WORKER_THREADS_LIMIT`](Self::SAFE_WORKER_THREADS_LIMIT).
    WorkerThreads,
}

impl SqliteLimit {
    /// SQLite's default for [`Length`](Self::Length).
    pub const DEFAULT_LENGTH_LIMIT: i32 = 1_000_000_000;
    /// Hardened value for [`Length`](Self::Length).
    pub const SAFE_LENGTH_LIMIT: i32 = 1_000_000;

    /// SQLite's default for [`SqlLength`](Self::SqlLength).
    pub const DEFAULT_SQL_LENGTH_LIMIT: i32 = 1_000_000_000;
    /// Hardened value for [`SqlLength`](Self::SqlLength).
    pub const SAFE_SQL_LENGTH_LIMIT: i32 = 100_000;

    /// SQLite's default for [`ColumnCount`](Self::ColumnCount).
    pub const DEFAULT_COLUMN_COUNT_LIMIT: i32 = 2_000;
    /// Hardened value for [`ColumnCount`](Self::ColumnCount).
    pub const SAFE_COLUMN_COUNT_LIMIT: i32 = 100;

    /// SQLite's default for [`ExprDepth`](Self::ExprDepth).
    pub const DEFAULT_EXPR_DEPTH_LIMIT: i32 = 1_000;
    /// Hardened value for [`ExprDepth`](Self::ExprDepth).
    pub const SAFE_EXPR_DEPTH_LIMIT: i32 = 10;

    /// SQLite's default for [`CompoundSelect`](Self::CompoundSelect).
    pub const DEFAULT_COMPOUND_SELECT_LIMIT: i32 = 500;
    /// Hardened value for [`CompoundSelect`](Self::CompoundSelect).
    pub const SAFE_COMPOUND_SELECT_LIMIT: i32 = 3;

    /// SQLite's default for [`VdbeOp`](Self::VdbeOp).
    pub const DEFAULT_VDBE_OP_LIMIT: i32 = 250_000_000;
    /// Hardened value for [`VdbeOp`](Self::VdbeOp).
    pub const SAFE_VDBE_OP_LIMIT: i32 = 25_000;

    /// SQLite's default for [`FunctionArg`](Self::FunctionArg).
    pub const DEFAULT_FUNCTION_ARG_LIMIT: i32 = 127;
    /// Hardened value for [`FunctionArg`](Self::FunctionArg).
    pub const SAFE_FUNCTION_ARG_LIMIT: i32 = 8;

    /// SQLite's default for [`Attached`](Self::Attached).
    pub const DEFAULT_ATTACHED_LIMIT: i32 = 10;
    /// Hardened value for [`Attached`](Self::Attached).
    pub const SAFE_ATTACHED_LIMIT: i32 = 0;

    /// SQLite's default for [`LikePatternLength`](Self::LikePatternLength).
    pub const DEFAULT_LIKE_PATTERN_LENGTH_LIMIT: i32 = 50_000;
    /// Hardened value for [`LikePatternLength`](Self::LikePatternLength).
    pub const SAFE_LIKE_PATTERN_LENGTH_LIMIT: i32 = 50;

    /// SQLite's published default for [`VariableNumber`](Self::VariableNumber).
    ///
    /// A particular build may compile a different maximum. The default
    /// `libsqlite3-sys` bundle sets `SQLITE_MAX_VARIABLE_NUMBER=250000`, so a
    /// connection's runtime default can exceed this published value.
    pub const DEFAULT_VARIABLE_NUMBER_LIMIT: i32 = 32_766;
    /// Hardened value for [`VariableNumber`](Self::VariableNumber).
    pub const SAFE_VARIABLE_NUMBER_LIMIT: i32 = 10;

    /// SQLite's default for [`TriggerDepth`](Self::TriggerDepth).
    pub const DEFAULT_TRIGGER_DEPTH_LIMIT: i32 = 1_000;
    /// Hardened value for [`TriggerDepth`](Self::TriggerDepth).
    pub const SAFE_TRIGGER_DEPTH_LIMIT: i32 = 10;

    /// SQLite's default for [`WorkerThreads`](Self::WorkerThreads).
    pub const DEFAULT_WORKER_THREADS_LIMIT: i32 = 0;
    /// Hardened value for [`WorkerThreads`](Self::WorkerThreads), equal to its
    /// default, so the recommended setter leaves it untouched.
    pub const SAFE_WORKER_THREADS_LIMIT: i32 = 0;

    /// Convert to the corresponding FFI constant value.
    pub(super) fn to_ffi(self) -> i32 {
        match self {
            SqliteLimit::Length => ffi::SQLITE_LIMIT_LENGTH,
            SqliteLimit::SqlLength => ffi::SQLITE_LIMIT_SQL_LENGTH,
            SqliteLimit::ColumnCount => ffi::SQLITE_LIMIT_COLUMN,
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
