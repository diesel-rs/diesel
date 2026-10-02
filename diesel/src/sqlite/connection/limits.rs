#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;

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

impl SqliteConnection {
    /// Set a runtime limit for this connection, returning its previous value.
    ///
    /// Lowering these limits is a way to harden a connection against untrusted
    /// SQL. See the [SQLite documentation](https://www.sqlite.org/c3ref/limit.html)
    /// for the meaning of each [`SqliteLimit`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let mut conn = SqliteConnection::establish(":memory:").unwrap();
    ///
    /// // Cap SQL statement length at 1 KiB, keeping the previous value.
    /// let previous = conn.set_limit(SqliteLimit::SqlLength, 1024);
    /// assert!(previous > 0);
    /// assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 1024);
    /// # }
    /// ```
    pub fn set_limit(&mut self, limit: SqliteLimit, value: i32) -> i32 {
        self.raw_connection.set_limit(limit, value)
    }

    /// Get the current value of a runtime limit for this connection.
    ///
    /// See the [SQLite documentation](https://www.sqlite.org/c3ref/limit.html)
    /// for the meaning of each [`SqliteLimit`].
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let conn = SqliteConnection::establish(":memory:").unwrap();
    /// assert!(conn.get_limit(SqliteLimit::SqlLength) > 0);
    /// # }
    /// ```
    pub fn get_limit(&self, limit: SqliteLimit) -> i32 {
        self.raw_connection.get_limit(limit)
    }

    /// Apply SQLite's recommended limits for hardening against untrusted SQL.
    ///
    /// These are the values from the "Untrusted SQL Inputs" table of SQLite's
    /// [security documentation](https://sqlite.org/security.html). They are
    /// intentionally restrictive, so call [`set_limit`](Self::set_limit)
    /// afterwards to relax any that are too aggressive for your application.
    ///
    /// | Limit | Value |
    /// |-------|-------|
    /// | `Length` | 1,000,000 |
    /// | `SqlLength` | 100,000 |
    /// | `ColumnCount` | 100 |
    /// | `ExprDepth` | 10 |
    /// | `CompoundSelect` | 3 |
    /// | `VdbeOp` | 25,000 |
    /// | `FunctionArg` | 8 |
    /// | `Attached` | 0 |
    /// | `LikePatternLength` | 50 |
    /// | `VariableNumber` | 10 |
    /// | `TriggerDepth` | 10 |
    ///
    /// The table's `PARSER_DEPTH` recommendation is omitted because it is a
    /// compile-time only setting with no runtime `sqlite3_limit()` category.
    /// `WorkerThreads` is left untouched (its default of 0 is already safe).
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() { run_test(); }
    /// # fn run_test() {
    /// use diesel::sqlite::SqliteLimit;
    ///
    /// let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_recommended_security_limits();
    /// assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 100_000);
    ///
    /// // Relax an individual limit that is too strict for this application.
    /// conn.set_limit(SqliteLimit::VariableNumber, 999);
    /// assert_eq!(conn.get_limit(SqliteLimit::VariableNumber), 999);
    /// # }
    /// ```
    pub fn set_recommended_security_limits(&mut self) {
        self.set_limit(SqliteLimit::Length, SqliteLimit::SAFE_LENGTH_LIMIT);
        self.set_limit(SqliteLimit::SqlLength, SqliteLimit::SAFE_SQL_LENGTH_LIMIT);
        self.set_limit(
            SqliteLimit::ColumnCount,
            SqliteLimit::SAFE_COLUMN_COUNT_LIMIT,
        );
        self.set_limit(SqliteLimit::ExprDepth, SqliteLimit::SAFE_EXPR_DEPTH_LIMIT);
        self.set_limit(
            SqliteLimit::CompoundSelect,
            SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT,
        );
        self.set_limit(SqliteLimit::VdbeOp, SqliteLimit::SAFE_VDBE_OP_LIMIT);
        self.set_limit(
            SqliteLimit::FunctionArg,
            SqliteLimit::SAFE_FUNCTION_ARG_LIMIT,
        );
        self.set_limit(SqliteLimit::Attached, SqliteLimit::SAFE_ATTACHED_LIMIT);
        self.set_limit(
            SqliteLimit::LikePatternLength,
            SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT,
        );
        self.set_limit(
            SqliteLimit::VariableNumber,
            SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT,
        );
        self.set_limit(
            SqliteLimit::TriggerDepth,
            SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[diesel_test_helper::test]
    fn set_limit_returns_previous_value() {
        let mut conn = connection();
        let original = conn.get_limit(SqliteLimit::SqlLength);

        // Setting a new value returns the old one, and a second set returns the
        // value installed by the first.
        assert_eq!(conn.set_limit(SqliteLimit::SqlLength, 1024), original);
        assert_eq!(conn.set_limit(SqliteLimit::SqlLength, 2048), 1024);
        assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 2048);
    }

    #[diesel_test_helper::test]
    fn get_limit_does_not_mutate() {
        let conn = connection();
        let first = conn.get_limit(SqliteLimit::ExprDepth);
        // Querying is implemented by passing -1 to sqlite3_limit, which must
        // leave the limit unchanged.
        assert!(first > 0);
        assert_eq!(conn.get_limit(SqliteLimit::ExprDepth), first);
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::Length, 100);

        assert!(
            crate::sql_query("SELECT length(randomblob(50))")
                .execute(&mut conn)
                .is_ok()
        );
        // A 500-byte blob exceeds the 100-byte row/value limit ("string or blob too big").
        assert!(
            crate::sql_query("SELECT length(randomblob(500))")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_column_count() {
        // A wide result set runs under the default column limit but fails once the limit is
        // lowered below its column count ("too many columns in result set").
        let wide = format!(
            "SELECT {}",
            (1..=30)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        let mut unconstrained = connection();
        assert!(crate::sql_query(&wide).execute(&mut unconstrained).is_ok());

        let mut conn = connection();
        conn.set_limit(SqliteLimit::ColumnCount, 10);
        assert!(crate::sql_query(&wide).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_expr_depth() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::ExprDepth, 5);

        assert!(crate::sql_query("SELECT 1+1").execute(&mut conn).is_ok());
        // A 40-deep addition tree exceeds the parse-tree depth of five.
        let deep = format!("SELECT {}1", "1+".repeat(40));
        assert!(crate::sql_query(&deep).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_compound_select() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::CompoundSelect, 2);

        assert!(
            crate::sql_query("SELECT 1 UNION SELECT 2")
                .execute(&mut conn)
                .is_ok()
        );
        // Five UNION terms exceed the limit of two ("too many terms in compound SELECT").
        assert!(
            crate::sql_query(
                "SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 UNION SELECT 5"
            )
            .execute(&mut conn)
            .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_vdbe_op() {
        // The same heavy statement runs under the default opcode budget but fails once that
        // budget is restricted to a tiny value (reported as SQLITE_NOMEM).
        let heavy = "SELECT count(*) FROM sqlite_master a, sqlite_master b, sqlite_master c";

        let mut unconstrained = connection();
        assert!(crate::sql_query(heavy).execute(&mut unconstrained).is_ok());

        let mut conn = connection();
        conn.set_limit(SqliteLimit::VdbeOp, 5);
        assert!(crate::sql_query(heavy).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_function_arg() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::FunctionArg, 3);

        assert!(
            crate::sql_query("SELECT max(1, 2, 3)")
                .execute(&mut conn)
                .is_ok()
        );
        // Eight arguments exceed the limit of three ("too many arguments on function max").
        assert!(
            crate::sql_query("SELECT max(1, 2, 3, 4, 5, 6, 7, 8)")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_attached() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::Attached, 0);

        // With zero attachments allowed, any ATTACH is rejected ("too many attached databases").
        assert!(
            crate::sql_query("ATTACH DATABASE ':memory:' AS aux_db")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_variable_number() {
        let mut conn = connection();
        // The published default sits below the bundled ceiling, so it is applied verbatim and
        // acts as the boundary: a parameter index at the limit is accepted, one past it is
        // rejected ("variable number must be between ?1 and ?N").
        conn.set_limit(
            SqliteLimit::VariableNumber,
            SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT,
        );
        let at_limit = format!("SELECT ?{}", SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT);
        let past_limit = format!(
            "SELECT ?{}",
            SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT as i64 + 1
        );
        assert!(crate::sql_query(&at_limit).execute(&mut conn).is_ok());
        assert!(crate::sql_query(&past_limit).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_trigger_depth() {
        use crate::connection::SimpleConnection;

        // A recursive trigger that terminates on its own at x = 100.
        let setup = "PRAGMA recursive_triggers = ON;\
             CREATE TABLE recur (x INTEGER);\
             CREATE TRIGGER recur_tr AFTER INSERT ON recur WHEN NEW.x < 100 \
             BEGIN INSERT INTO recur VALUES (NEW.x + 1); END;";

        // Under the default depth the recursion completes.
        let mut unconstrained = connection();
        unconstrained.batch_execute(setup).unwrap();
        assert!(
            crate::sql_query("INSERT INTO recur VALUES (1)")
                .execute(&mut unconstrained)
                .is_ok()
        );

        // A tiny depth limit is hit before the recursion can terminate
        // ("too many levels of trigger recursion").
        let mut conn = connection();
        conn.set_limit(SqliteLimit::TriggerDepth, 3);
        conn.batch_execute(setup).unwrap();
        assert!(
            crate::sql_query("INSERT INTO recur VALUES (1)")
                .execute(&mut conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn worker_threads_limit_has_no_runtime_error_path() {
        // Unlike the other categories, WorkerThreads only caps the number of auxiliary sort
        // threads a statement may start. Lowering it never raises an error, it only affects
        // performance. There is therefore no enforcement failure to assert, only that the value
        // is applied and ordinary queries keep working.
        let mut conn = connection();
        conn.set_limit(SqliteLimit::WorkerThreads, 0);
        assert_eq!(conn.get_limit(SqliteLimit::WorkerThreads), 0);
        assert!(crate::sql_query("SELECT 1").execute(&mut conn).is_ok());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_sql_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::SqlLength, 20);

        // A statement longer than 20 bytes is rejected by SQLite.
        let result =
            crate::sql_query("SELECT * FROM sqlite_master WHERE type = 'table'").execute(&mut conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_enforces_like_pattern_length() {
        let mut conn = connection();
        conn.set_limit(SqliteLimit::LikePatternLength, 100);

        assert!(
            crate::sql_query("SELECT 'test' LIKE 'te%'")
                .execute(&mut conn)
                .is_ok()
        );

        let long_pattern = "%".repeat(200);
        let query = format!("SELECT 'test' LIKE '{long_pattern}'");
        assert!(crate::sql_query(&query).execute(&mut conn).is_err());
    }

    #[diesel_test_helper::test]
    fn set_limit_clamps_above_compile_time_maximum() {
        let mut conn = connection();
        // SQLite clamps a requested value to its hard compile-time ceiling
        // rather than accepting it verbatim.
        conn.set_limit(SqliteLimit::Length, i32::MAX);
        let clamped = conn.get_limit(SqliteLimit::Length);
        assert!(clamped > 0 && clamped < i32::MAX);
    }

    #[diesel_test_helper::test]
    fn set_recommended_security_limits_applies_documented_table() {
        let mut conn = connection();
        conn.set_recommended_security_limits();

        assert_eq!(conn.get_limit(SqliteLimit::Length), 1_000_000);
        assert_eq!(conn.get_limit(SqliteLimit::SqlLength), 100_000);
        assert_eq!(conn.get_limit(SqliteLimit::ColumnCount), 100);
        assert_eq!(conn.get_limit(SqliteLimit::ExprDepth), 10);
        assert_eq!(conn.get_limit(SqliteLimit::CompoundSelect), 3);
        assert_eq!(conn.get_limit(SqliteLimit::VdbeOp), 25_000);
        assert_eq!(conn.get_limit(SqliteLimit::FunctionArg), 8);
        assert_eq!(conn.get_limit(SqliteLimit::Attached), 0);
        assert_eq!(conn.get_limit(SqliteLimit::LikePatternLength), 50);
        assert_eq!(conn.get_limit(SqliteLimit::VariableNumber), 10);
        assert_eq!(conn.get_limit(SqliteLimit::TriggerDepth), 10);
    }

    #[diesel_test_helper::test]
    fn safe_limit_constants_do_not_exceed_defaults() {
        // The hardened value for each category is a tightening of SQLite's published default, so
        // it must never be larger. This is asserted instead of comparing the `DEFAULT_*`
        // constants to a fresh connection, because the runtime default of categories such as
        // `FunctionArg` and `VariableNumber` is build-dependent (the bundled libsqlite3-sys
        // raises several of them), while these published constants are fixed.
        let pairs = [
            (
                SqliteLimit::SAFE_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_SQL_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_SQL_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_COLUMN_COUNT_LIMIT,
                SqliteLimit::DEFAULT_COLUMN_COUNT_LIMIT,
            ),
            (
                SqliteLimit::SAFE_EXPR_DEPTH_LIMIT,
                SqliteLimit::DEFAULT_EXPR_DEPTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT,
                SqliteLimit::DEFAULT_COMPOUND_SELECT_LIMIT,
            ),
            (
                SqliteLimit::SAFE_VDBE_OP_LIMIT,
                SqliteLimit::DEFAULT_VDBE_OP_LIMIT,
            ),
            (
                SqliteLimit::SAFE_FUNCTION_ARG_LIMIT,
                SqliteLimit::DEFAULT_FUNCTION_ARG_LIMIT,
            ),
            (
                SqliteLimit::SAFE_ATTACHED_LIMIT,
                SqliteLimit::DEFAULT_ATTACHED_LIMIT,
            ),
            (
                SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT,
                SqliteLimit::DEFAULT_LIKE_PATTERN_LENGTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT,
                SqliteLimit::DEFAULT_VARIABLE_NUMBER_LIMIT,
            ),
            (
                SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT,
                SqliteLimit::DEFAULT_TRIGGER_DEPTH_LIMIT,
            ),
            (
                SqliteLimit::SAFE_WORKER_THREADS_LIMIT,
                SqliteLimit::DEFAULT_WORKER_THREADS_LIMIT,
            ),
        ];
        for (safe, default) in pairs {
            assert!(
                safe <= default,
                "safe value {safe} exceeds default {default}"
            );
        }
    }

    #[diesel_test_helper::test]
    fn safe_limit_constants_match_recommended_setter() {
        let mut conn = connection();
        conn.set_recommended_security_limits();

        assert_eq!(
            conn.get_limit(SqliteLimit::Length),
            SqliteLimit::SAFE_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::SqlLength),
            SqliteLimit::SAFE_SQL_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::ColumnCount),
            SqliteLimit::SAFE_COLUMN_COUNT_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::ExprDepth),
            SqliteLimit::SAFE_EXPR_DEPTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::CompoundSelect),
            SqliteLimit::SAFE_COMPOUND_SELECT_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::VdbeOp),
            SqliteLimit::SAFE_VDBE_OP_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::FunctionArg),
            SqliteLimit::SAFE_FUNCTION_ARG_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::Attached),
            SqliteLimit::SAFE_ATTACHED_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::LikePatternLength),
            SqliteLimit::SAFE_LIKE_PATTERN_LENGTH_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::VariableNumber),
            SqliteLimit::SAFE_VARIABLE_NUMBER_LIMIT
        );
        assert_eq!(
            conn.get_limit(SqliteLimit::TriggerDepth),
            SqliteLimit::SAFE_TRIGGER_DEPTH_LIMIT
        );
        // The recommended setter leaves `WorkerThreads` untouched because its default is already
        // safe, so assert that the documented safe value matches what the connection reports.
        assert_eq!(
            conn.get_limit(SqliteLimit::WorkerThreads),
            SqliteLimit::SAFE_WORKER_THREADS_LIMIT
        );
    }
}
