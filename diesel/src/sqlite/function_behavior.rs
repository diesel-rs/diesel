//! SQLite function behavior flags for custom SQL functions.

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

bitflags::bitflags! {
    /// Flags controlling SQLite custom function behavior.
    ///
    /// These flags are passed to `sqlite3_create_function_v2()` and control
    /// how SQLite treats the function in various contexts.
    ///
    /// # Availability
    ///
    /// - [`DETERMINISTIC`][Self::DETERMINISTIC]: SQLite 3.8.3 (2014-02)
    /// - [`DIRECTONLY`][Self::DIRECTONLY], [`INNOCUOUS`][Self::INNOCUOUS]: SQLite 3.31.0 (2020-01)
    /// - [`SUBTYPE`][Self::SUBTYPE]: SQLite 3.30.0 (2019-10)
    ///
    /// # Security Considerations
    ///
    /// When using [`SqliteConnection::set_trusted_schema(false)`][crate::sqlite::SqliteConnection::set_trusted_schema]
    /// for security hardening, custom functions must be marked with [`INNOCUOUS`][Self::INNOCUOUS]
    /// to be callable from views, triggers, CHECK constraints, DEFAULT expressions,
    /// generated columns, and expression indexes.
    ///
    /// Conversely, functions with side effects or that expose sensitive state
    /// should be marked with [`DIRECTONLY`][Self::DIRECTONLY] to prevent them from being called
    /// via malicious schema objects in untrusted database files.
    ///
    /// # Example
    ///
    /// ```rust
    /// use diesel::sqlite::SqliteFunctionBehavior;
    ///
    /// // Deterministic function (most common case)
    /// let flags = SqliteFunctionBehavior::DETERMINISTIC;
    /// assert!(flags.contains(SqliteFunctionBehavior::DETERMINISTIC));
    ///
    /// // Non-deterministic function (e.g., random())
    /// let flags = SqliteFunctionBehavior::empty();
    /// assert!(flags.is_empty());
    ///
    /// // Safe for use in untrusted schemas (views, triggers, etc.)
    /// let flags = SqliteFunctionBehavior::DETERMINISTIC | SqliteFunctionBehavior::INNOCUOUS;
    /// assert!(flags.contains(SqliteFunctionBehavior::DETERMINISTIC));
    /// assert!(flags.contains(SqliteFunctionBehavior::INNOCUOUS));
    ///
    /// // Has side effects, block from schema objects
    /// let flags = SqliteFunctionBehavior::DIRECTONLY;
    /// assert!(flags.contains(SqliteFunctionBehavior::DIRECTONLY));
    /// ```
    ///
    /// When registering a custom SQL function:
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// use diesel::sqlite::SqliteFunctionBehavior;
    /// use diesel::prelude::*;
    ///
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> diesel::result::QueryResult<()> {
    /// #     let mut conn = establish_connection();
    /// // Use SqliteFunctionBehavior when registering SQL functions
    /// conn.register_sql_function::<diesel::sql_types::Text, diesel::sql_types::Text, _, _, _>(
    ///     "my_upper",
    ///     SqliteFunctionBehavior::DETERMINISTIC | SqliteFunctionBehavior::INNOCUOUS,
    ///     |x: String| x.to_uppercase(),
    /// )?;
    /// #     Ok(())
    /// # }
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SqliteFunctionBehavior: i32 {
        /// The function always returns the same result given the same inputs
        /// within a single SQL statement.
        ///
        /// This allows SQLite to optimize by caching results and factoring
        /// the function out of inner loops. Most pure functions should use this flag.
        ///
        /// # Availability
        ///
        /// Requires SQLite 3.8.3 (2014-02) or later.
        ///
        /// # Example
        ///
        /// `abs(x)` is deterministic. `random()` is not.
        const DETERMINISTIC = ffi::SQLITE_DETERMINISTIC;

        /// The function cannot be called from schema objects (views, triggers,
        /// CHECK constraints, DEFAULT expressions, generated columns, or
        /// expression indexes).
        ///
        /// Use this for functions that:
        /// - Have side effects (network, file I/O, logging, etc.)
        /// - Return sensitive application state
        /// - Should not be triggered by opening an untrusted database
        ///
        /// # Availability
        ///
        /// Requires SQLite 3.31.0 (2020-01) or later.
        ///
        /// # Security Recommendation
        ///
        /// Mark all functions with side effects or that expose internal state
        /// with `DIRECTONLY` to prevent schema injection attacks.
        const DIRECTONLY = ffi::SQLITE_DIRECTONLY;

        /// The function is safe to call from untrusted schema contexts.
        ///
        /// When [`SqliteConnection::set_trusted_schema(false)`][crate::sqlite::SqliteConnection::set_trusted_schema]
        /// is set, only functions marked `INNOCUOUS` can be called from schema
        /// objects (views, triggers, CHECK constraints, DEFAULT expressions,
        /// generated columns, expression indexes).
        ///
        /// # Availability
        ///
        /// Requires SQLite 3.31.0 (2020-01) or later.
        ///
        /// # Safety Requirements
        ///
        /// **Only mark a function as INNOCUOUS if it:**
        /// - Has no side effects
        /// - Does not reveal internal application state
        /// - Output depends solely on its input parameters
        ///
        /// # Security Warning
        ///
        /// Incorrectly marking a function as `INNOCUOUS` can create security
        /// vulnerabilities when processing untrusted database files. An attacker
        /// could craft a database with malicious views or triggers that invoke
        /// your function in unexpected ways.
        const INNOCUOUS = ffi::SQLITE_INNOCUOUS;

        /// The function may call `sqlite3_value_subtype()` to inspect the
        /// subtype of its arguments.
        ///
        /// # Availability
        ///
        /// Requires SQLite 3.30.0 (2019-10) or later.
        const SUBTYPE = ffi::SQLITE_SUBTYPE;
    }
}

impl Default for SqliteFunctionBehavior {
    /// Returns [`DETERMINISTIC`][Self::DETERMINISTIC], matching the previous
    /// behavior of `register_impl`.
    fn default() -> Self {
        Self::DETERMINISTIC
    }
}

impl SqliteFunctionBehavior {
    /// Returns the raw flags value including UTF8 encoding flag.
    ///
    /// This is used internally when calling `sqlite3_create_function_v2`.
    pub(crate) fn to_flags(self) -> i32 {
        ffi::SQLITE_UTF8 | self.bits()
    }
}
