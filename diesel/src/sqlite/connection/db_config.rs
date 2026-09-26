#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
extern crate libsqlite3_sys as ffi;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use sqlite_wasm_rs as ffi;

use super::SqliteConnection;
use super::raw;
use crate::result::QueryResult;

impl SqliteConnection {
    /// Enable or disable SQLite defensive mode.
    ///
    /// When enabled, defensive mode prevents direct writes to shadow tables
    /// (FTS5, R-Tree, etc.), dangerous PRAGMAs like `writable_schema`,
    /// `sqlite3_deserialize()` from opening unsafe database images, and other
    /// potentially dangerous operations. Enable it for any connection that may
    /// process untrusted data. It is the single most important hardening flag.
    ///
    /// Requires SQLite 3.26.0 or later, otherwise returns an error.
    ///
    /// # Security Hardening Recipe
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_defensive(true).unwrap();
    /// conn.set_trusted_schema(false).unwrap();
    /// conn.set_recommended_security_limits();
    /// # }
    /// ```
    ///
    /// Extension loading is off by default. Enable it only when needed via
    /// [`with_load_extension_enabled`][Self::with_load_extension_enabled]. See
    /// [`set_recommended_security_limits`][Self::set_recommended_security_limits]
    /// to harden the SQLite resource limits as well.
    pub fn set_defensive(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DEFENSIVE, enabled)
    }

    /// Check if defensive mode is enabled.
    ///
    /// See [`set_defensive`][Self::set_defensive] for details.
    pub fn is_defensive(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DEFENSIVE)
    }

    /// Enable or disable trusted schema mode.
    ///
    /// When disabled (untrusted), SQL functions called from schema objects
    /// (views, triggers, CHECK constraints, DEFAULT expressions, generated
    /// columns, expression indexes) are restricted to those marked
    /// [`INNOCUOUS`][crate::sqlite::SqliteFunctionBehavior::INNOCUOUS]. Disable
    /// it when opening database files from untrusted sources, and register your
    /// custom functions with appropriate
    /// [`SqliteFunctionBehavior`][crate::sqlite::SqliteFunctionBehavior] flags.
    ///
    /// Requires SQLite 3.31.0 or later, otherwise returns an error.
    pub fn set_trusted_schema(&mut self, trusted: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA, trusted)
    }

    /// Check if trusted schema mode is enabled.
    ///
    /// See [`set_trusted_schema`][Self::set_trusted_schema] for details.
    pub fn is_trusted_schema(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
    }

    /// Runs the given closure with the `load_extension()` SQL function enabled,
    /// disabling it again afterwards.
    ///
    /// This controls the [`load_extension()`](https://www.sqlite.org/lang_corefunc.html#load_extension)
    /// **SQL function**, not the `sqlite3_load_extension()` C API (which Diesel
    /// does not expose). Extension loading is off by default, and scoping it to a
    /// closure keeps the window in which it is enabled as small as possible.
    ///
    /// Requires SQLite 3.13.0 or later, otherwise returns an error. Has no effect
    /// if SQLite was compiled with `SQLITE_OMIT_LOAD_EXTENSION`.
    ///
    /// # Panics
    ///
    /// If `f` panics, extension loading is disabled again before the panic
    /// resumes. no-std builds cannot catch the unwind, so there the flag is
    /// restored only on a normal return.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     let mut conn = SqliteConnection::establish(":memory:").unwrap();
    /// let result: QueryResult<()> = conn.with_load_extension_enabled(|_conn| Ok(()));
    /// result.unwrap();
    /// # }
    /// ```
    pub fn with_load_extension_enabled<R, E>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E>
    where
        E: From<crate::result::Error>,
    {
        self.set_load_extension_enabled(true)?;

        // On std builds, catch a panic from `f` so extension loading is restored
        // before the panic is resumed. no-std cannot catch unwinding, so there
        // the flag is restored only on a normal return.
        #[cfg(feature = "std")]
        {
            match std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| f(self))) {
                Ok(r) => {
                    self.set_load_extension_enabled(false)?;
                    r
                }
                Err(panic) => {
                    let _ = self.set_load_extension_enabled(false);
                    std::panic::resume_unwind(panic);
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let r = f(self);
            self.set_load_extension_enabled(false)?;
            r
        }
    }

    fn set_load_extension_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION, enabled)
    }

    #[cfg(test)]
    fn is_load_extension_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION)
    }

    /// Enable or disable the `fts3_tokenizer()` SQL function.
    ///
    /// The [`fts3_tokenizer()`](https://www.sqlite.org/fts3.html#f3tknzr) function
    /// allows overloading the default FTS3/FTS4 tokenizer, which can be exploited
    /// if an attacker can execute arbitrary SQL. Disable it unless you need custom
    /// FTS3 tokenizers.
    ///
    /// Requires SQLite 3.12.0 or later, otherwise returns an error.
    pub fn set_fts3_tokenizer_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER, enabled)
    }

    /// Check if the `fts3_tokenizer()` SQL function is enabled.
    ///
    /// See [`set_fts3_tokenizer_enabled`][Self::set_fts3_tokenizer_enabled] for details.
    pub fn is_fts3_tokenizer_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FTS3_TOKENIZER)
    }

    /// Enable or disable direct writes to `sqlite_master`.
    ///
    /// When enabled, allows direct modification of the `sqlite_master` table,
    /// which can corrupt the database if misused. Keep it disabled unless you
    /// need to repair or modify the schema directly. Defensive mode
    /// ([`set_defensive`][Self::set_defensive]) also prevents this.
    ///
    /// Requires SQLite 3.28.0 or later, otherwise returns an error.
    pub fn set_writable_schema(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_WRITABLE_SCHEMA, enabled)
    }

    /// Check if direct writes to `sqlite_master` are enabled.
    ///
    /// See [`set_writable_schema`][Self::set_writable_schema] for details.
    pub fn is_writable_schema(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_WRITABLE_SCHEMA)
    }

    /// Enable or disable ATTACH from creating new database files.
    ///
    /// When disabled, [`ATTACH`](https://www.sqlite.org/lang_attach.html) can only
    /// open existing database files, not create new ones. Disable it where
    /// database file creation should be restricted.
    ///
    /// Requires SQLite 3.49.0 or later, otherwise returns an error.
    pub fn set_attach_create_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE, enabled)
    }

    /// Check if ATTACH can create new database files.
    ///
    /// See [`set_attach_create_enabled`][Self::set_attach_create_enabled] for details.
    pub fn is_attach_create_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_CREATE)
    }

    /// Enable or disable ATTACH from opening databases in write mode.
    ///
    /// When disabled, all attached databases are opened as read-only. Disable it
    /// to restrict write access to attached databases.
    ///
    /// Requires SQLite 3.49.0 or later, otherwise returns an error.
    pub fn set_attach_write_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE, enabled)
    }

    /// Check if ATTACH can open databases in write mode.
    ///
    /// See [`set_attach_write_enabled`][Self::set_attach_write_enabled] for details.
    pub fn is_attach_write_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(raw::SQLITE_DBCONFIG_ENABLE_ATTACH_WRITE)
    }

    /// Enable or disable trigger execution.
    ///
    /// When disabled, triggers will not fire for any DML operations.
    ///
    /// Requires SQLite 3.8.7 or later, otherwise returns an error.
    pub fn set_triggers_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_TRIGGER, enabled)
    }

    /// Check if trigger execution is enabled.
    ///
    /// See [`set_triggers_enabled`][Self::set_triggers_enabled] for details.
    pub fn are_triggers_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_TRIGGER)
    }

    /// Enable or disable view expansion.
    ///
    /// When disabled, queries against views will fail.
    ///
    /// Requires SQLite 3.30.0 or later, otherwise returns an error.
    pub fn set_views_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_VIEW, enabled)
    }

    /// Check if view expansion is enabled.
    ///
    /// See [`set_views_enabled`][Self::set_views_enabled] for details.
    pub fn are_views_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_VIEW)
    }

    /// Enable or disable foreign key constraint enforcement.
    ///
    /// This is equivalent to `PRAGMA foreign_keys = ON/OFF`.
    ///
    /// Requires SQLite 3.8.7 or later, otherwise returns an error.
    pub fn set_foreign_keys_enabled(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FKEY, enabled)
    }

    /// Check if foreign key constraints are enabled.
    ///
    /// See [`set_foreign_keys_enabled`][Self::set_foreign_keys_enabled] for details.
    pub fn are_foreign_keys_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_ENABLE_FKEY)
    }

    /// Enable or disable double-quoted strings in DML statements.
    ///
    /// When enabled, double-quoted strings are interpreted as string literals
    /// rather than identifiers, a legacy behavior that can cause issues. Disable
    /// it for stricter SQL compliance.
    ///
    /// Requires SQLite 3.29.0 or later, otherwise returns an error.
    pub fn set_double_quoted_strings_dml(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML, enabled)
    }

    /// Check if double-quoted strings in DML are enabled.
    ///
    /// See [`set_double_quoted_strings_dml`][Self::set_double_quoted_strings_dml] for details.
    pub fn are_double_quoted_strings_dml_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DML)
    }

    /// Enable or disable double-quoted strings in DDL statements.
    ///
    /// When enabled, double-quoted strings are interpreted as string literals
    /// rather than identifiers, a legacy behavior that can cause issues. Disable
    /// it for stricter SQL compliance.
    ///
    /// Requires SQLite 3.29.0 or later, otherwise returns an error.
    pub fn set_double_quoted_strings_ddl(&mut self, enabled: bool) -> QueryResult<()> {
        self.raw_connection
            .set_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DDL, enabled)
    }

    /// Check if double-quoted strings in DDL are enabled.
    ///
    /// See [`set_double_quoted_strings_ddl`][Self::set_double_quoted_strings_ddl] for details.
    pub fn are_double_quoted_strings_ddl_enabled(&self) -> QueryResult<bool> {
        self.raw_connection
            .get_db_config_bool(ffi::SQLITE_DBCONFIG_DQS_DDL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::Text;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    // ---- db_config tests ----

    #[diesel_test_helper::test]
    fn db_config_defensive_roundtrip() {
        let conn = &mut connection();
        conn.set_defensive(true).unwrap();
        assert!(conn.is_defensive().unwrap());
        conn.set_defensive(false).unwrap();
        assert!(!conn.is_defensive().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_trusted_schema_roundtrip() {
        let conn = &mut connection();
        conn.set_trusted_schema(false).unwrap();
        assert!(!conn.is_trusted_schema().unwrap());
        conn.set_trusted_schema(true).unwrap();
        assert!(conn.is_trusted_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_with_load_extension_enabled_scopes_the_flag() {
        let conn = &mut connection();
        conn.with_load_extension_enabled(|conn| {
            // Enabled for the duration of the closure.
            assert!(conn.is_load_extension_enabled().unwrap());
            QueryResult::Ok(())
        })
        .unwrap();
        // Disabled again afterwards.
        assert!(!conn.is_load_extension_enabled().unwrap());
    }

    #[cfg(all(
        feature = "std",
        not(all(target_family = "wasm", target_os = "unknown"))
    ))]
    #[diesel_test_helper::test]
    fn with_load_extension_enabled_disables_after_panic() {
        let conn = &mut connection();
        let outcome = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            conn.with_load_extension_enabled(|_conn| -> QueryResult<()> {
                panic!("boom inside closure");
            })
        }));
        assert!(outcome.is_err(), "panic should propagate");
        assert!(
            !conn.is_load_extension_enabled().unwrap(),
            "extension loading must be disabled again after a panic"
        );
    }

    #[diesel_test_helper::test]
    fn db_config_triggers_roundtrip() {
        let conn = &mut connection();
        conn.set_triggers_enabled(false).unwrap();
        assert!(!conn.are_triggers_enabled().unwrap());
        conn.set_triggers_enabled(true).unwrap();
        assert!(conn.are_triggers_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_views_roundtrip() {
        let conn = &mut connection();
        conn.set_views_enabled(false).unwrap();
        assert!(!conn.are_views_enabled().unwrap());
        conn.set_views_enabled(true).unwrap();
        assert!(conn.are_views_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_foreign_keys_roundtrip() {
        let conn = &mut connection();
        conn.set_foreign_keys_enabled(true).unwrap();
        assert!(conn.are_foreign_keys_enabled().unwrap());
        conn.set_foreign_keys_enabled(false).unwrap();
        assert!(!conn.are_foreign_keys_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_dqs_dml_roundtrip() {
        let conn = &mut connection();
        conn.set_double_quoted_strings_dml(false).unwrap();
        assert!(!conn.are_double_quoted_strings_dml_enabled().unwrap());
        conn.set_double_quoted_strings_dml(true).unwrap();
        assert!(conn.are_double_quoted_strings_dml_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_dqs_ddl_roundtrip() {
        let conn = &mut connection();
        conn.set_double_quoted_strings_ddl(false).unwrap();
        assert!(!conn.are_double_quoted_strings_ddl_enabled().unwrap());
        conn.set_double_quoted_strings_ddl(true).unwrap();
        assert!(conn.are_double_quoted_strings_ddl_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_fts3_tokenizer_roundtrip() {
        let conn = &mut connection();
        conn.set_fts3_tokenizer_enabled(false).unwrap();
        assert!(!conn.is_fts3_tokenizer_enabled().unwrap());
        conn.set_fts3_tokenizer_enabled(true).unwrap();
        assert!(conn.is_fts3_tokenizer_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_writable_schema_roundtrip() {
        let conn = &mut connection();
        conn.set_writable_schema(false).unwrap();
        assert!(!conn.is_writable_schema().unwrap());
        conn.set_writable_schema(true).unwrap();
        assert!(conn.is_writable_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_attach_create_roundtrip() {
        let conn = &mut connection();
        // ATTACH_CREATE requires SQLite 3.46.0+; skip if unsupported
        if conn.set_attach_create_enabled(false).is_err() {
            return;
        }
        assert!(!conn.is_attach_create_enabled().unwrap());
        conn.set_attach_create_enabled(true).unwrap();
        assert!(conn.is_attach_create_enabled().unwrap());
    }

    #[diesel_test_helper::test]
    fn db_config_attach_write_roundtrip() {
        let conn = &mut connection();
        // ATTACH_WRITE requires SQLite 3.46.0+; skip if unsupported
        if conn.set_attach_write_enabled(false).is_err() {
            return;
        }
        assert!(!conn.is_attach_write_enabled().unwrap());
        conn.set_attach_write_enabled(true).unwrap();
        assert!(conn.is_attach_write_enabled().unwrap());
    }

    // ---- behavioral db_config tests ----

    #[diesel_test_helper::test]
    fn defensive_mode_blocks_writable_schema() {
        let conn = &mut connection();
        conn.set_defensive(true).unwrap();
        // In defensive mode, writable_schema should remain off even if we try to set it
        let _ = crate::sql_query("PRAGMA writable_schema = ON").execute(conn);
        assert!(!conn.is_writable_schema().unwrap());
    }

    #[diesel_test_helper::test]
    fn foreign_keys_enabled_enforces_constraints() {
        let conn = &mut connection();
        conn.set_foreign_keys_enabled(true).unwrap();

        crate::sql_query("CREATE TABLE parent (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query(
            "CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id))",
        )
        .execute(conn)
        .unwrap();

        // Insert a child row with no matching parent — should fail with FK enabled
        let result =
            crate::sql_query("INSERT INTO child (id, parent_id) VALUES (1, 999)").execute(conn);
        assert!(result.is_err());
    }

    #[diesel_test_helper::test]
    fn views_disabled_blocks_view_queries() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE base (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO base (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE VIEW base_view AS SELECT id FROM base")
            .execute(conn)
            .unwrap();

        // Enabled (default): the view can be queried.
        conn.set_views_enabled(true).unwrap();
        assert!(
            crate::sql_query("SELECT id FROM base_view")
                .execute(conn)
                .is_ok()
        );

        // Disabled: queries that reference the view fail.
        conn.set_views_enabled(false).unwrap();
        assert!(
            crate::sql_query("SELECT id FROM base_view")
                .execute(conn)
                .is_err()
        );
    }

    #[diesel_test_helper::test]
    fn triggers_disabled_prevents_firing() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE source (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TABLE trigger_log (n INTEGER)")
            .execute(conn)
            .unwrap();
        crate::sql_query("CREATE TRIGGER log_insert AFTER INSERT ON source BEGIN INSERT INTO trigger_log (n) VALUES (1); END")
            .execute(conn)
            .unwrap();

        // Disabled: inserting into `source` must not fire the trigger.
        conn.set_triggers_enabled(false).unwrap();
        crate::sql_query("INSERT INTO source (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM trigger_log")
            .get_result(conn)
            .unwrap();
        assert_eq!(0, count, "trigger should not fire while disabled");

        // Enabled: the trigger fires and writes one row.
        conn.set_triggers_enabled(true).unwrap();
        crate::sql_query("INSERT INTO source (id) VALUES (2)")
            .execute(conn)
            .unwrap();
        let count: i64 = sql::<crate::sql_types::BigInt>("SELECT COUNT(*) FROM trigger_log")
            .get_result(conn)
            .unwrap();
        assert_eq!(1, count, "trigger should fire while enabled");
    }

    #[diesel_test_helper::test]
    fn dqs_dml_controls_double_quoted_string_literals() {
        let conn = &mut connection();

        // Disabled: a double-quoted token in DML is parsed as an identifier, so a
        // bare `"text"` that is not a column errors.
        conn.set_double_quoted_strings_dml(false).unwrap();
        let disabled = sql::<Text>(r#"SELECT "bare_token""#).get_result::<String>(conn);
        assert!(disabled.is_err());

        // Enabled: the same token is accepted as a string literal.
        conn.set_double_quoted_strings_dml(true).unwrap();
        let enabled = sql::<Text>(r#"SELECT "bare_token""#).get_result::<String>(conn);
        assert_eq!(Ok("bare_token".to_owned()), enabled);
    }

    #[diesel_test_helper::test]
    fn dqs_ddl_controls_double_quoted_string_literals() {
        let conn = &mut connection();

        // Disabled: a double-quoted token in a CHECK constraint is parsed as an
        // identifier. As there is no such column, creating the table errors.
        conn.set_double_quoted_strings_ddl(false).unwrap();
        let disabled =
            crate::sql_query(r#"CREATE TABLE dqs_off (name TEXT, CHECK (name <> "not_a_column"))"#)
                .execute(conn);
        assert!(disabled.is_err());

        // Enabled: the same token is accepted as a string literal, so the CHECK
        // constraint (and the table) are created successfully.
        conn.set_double_quoted_strings_ddl(true).unwrap();
        let enabled =
            crate::sql_query(r#"CREATE TABLE dqs_on (name TEXT, CHECK (name <> "not_a_column"))"#)
                .execute(conn);
        assert!(enabled.is_ok());
    }

    #[diesel_test_helper::test]
    fn writable_schema_controls_direct_sqlite_master_writes() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE protected (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();

        let update =
            "UPDATE sqlite_master SET sql = sql WHERE type = 'table' AND name = 'protected'";

        // Disabled (default): a direct write to sqlite_master is rejected.
        conn.set_writable_schema(false).unwrap();
        assert!(crate::sql_query(update).execute(conn).is_err());

        // Enabled: the same write is permitted.
        conn.set_writable_schema(true).unwrap();
        assert!(crate::sql_query(update).execute(conn).is_ok());
    }

    #[diesel_test_helper::test]
    fn fts3_tokenizer_disabled_blocks_the_function() {
        let conn = &mut connection();

        // Enable first to detect whether FTS3 is compiled into this SQLite build.
        conn.set_fts3_tokenizer_enabled(true).unwrap();
        let enabled = sql::<crate::sql_types::Binary>("SELECT fts3_tokenizer('simple')")
            .get_result::<Vec<u8>>(conn);
        if enabled.is_err() {
            // FTS3 is not available in this build, so there is nothing to assert.
            return;
        }

        // Disabled: the `fts3_tokenizer()` SQL function is no longer callable.
        conn.set_fts3_tokenizer_enabled(false).unwrap();
        let disabled = sql::<crate::sql_types::Binary>("SELECT fts3_tokenizer('simple')")
            .get_result::<Vec<u8>>(conn);
        assert!(disabled.is_err());
    }

    // These ATTACH tests need a real filesystem (temp files), which is not
    // available on the wasm target, where SQLite is in-memory only.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn temp_db_path(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        (dir, path)
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_create_disabled_blocks_new_database_files() {
        let conn = &mut connection();

        // The ATTACH_CREATE option was added in SQLite 3.49.0; skip on older
        // libraries (e.g. the system SQLite on the Ubuntu 24.04 CI runners).
        if conn.set_attach_create_enabled(false).is_err() {
            return;
        }

        let (_dir, path) = temp_db_path("create.db");

        // Disabled: attaching a path that does not exist yet must fail.
        assert!(
            conn.attach_database(path.to_str().unwrap(), "aux_create")
                .is_err()
        );

        // Enabled: the same ATTACH now creates and opens the file.
        conn.set_attach_create_enabled(true).unwrap();
        conn.attach_database(path.to_str().unwrap(), "aux_create")
            .unwrap();
        conn.detach_database("aux_create").unwrap();
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_write_disabled_opens_attached_databases_read_only() {
        let conn = &mut connection();

        // The ATTACH_WRITE option was added in SQLite 3.49.0; skip on older
        // libraries (e.g. the system SQLite on the Ubuntu 24.04 CI runners).
        // This guard also leaves ATTACH_WRITE disabled for the first check below.
        if conn.set_attach_write_enabled(false).is_err() {
            return;
        }

        // Seed an existing on-disk database with a table to write into.
        let (_dir, path) = temp_db_path("write.db");
        {
            let mut seed = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
            crate::sql_query("CREATE TABLE t (id INTEGER)")
                .execute(&mut seed)
                .unwrap();
        }

        // Disabled: the attached database is opened read-only, so writes fail.
        conn.attach_database(path.to_str().unwrap(), "aux_write")
            .unwrap();
        assert!(
            crate::sql_query("INSERT INTO aux_write.t (id) VALUES (1)")
                .execute(conn)
                .is_err()
        );
        conn.detach_database("aux_write").unwrap();

        // Enabled: the attached database is writable again.
        conn.set_attach_write_enabled(true).unwrap();
        conn.attach_database(path.to_str().unwrap(), "aux_write")
            .unwrap();
        crate::sql_query("INSERT INTO aux_write.t (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        conn.detach_database("aux_write").unwrap();
    }
}
