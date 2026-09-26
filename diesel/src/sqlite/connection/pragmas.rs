use super::SqliteConnection;
use crate::query_builder::{AstPass, Query, QueryFragment, QueryId};
use crate::query_dsl::RunQueryDslSupport;
use crate::result::QueryResult;
use crate::sqlite::Sqlite;
use alloc::string::ToString;
use core::marker::PhantomData;

/// The `auto_vacuum` mode of a database, controlling whether and when SQLite
/// reclaims freed pages back to the file.
///
/// The mode is stored in the database file, not the connection. [`Full`] and
/// [`Incremental`] can be switched between at any time, but changing from or to
/// [`None`] only takes effect on a database with no tables yet, or after a
/// subsequent `VACUUM` rewrites the file.
///
/// [`None`]: AutoVacuumMode::None
/// [`Full`]: AutoVacuumMode::Full
/// [`Incremental`]: AutoVacuumMode::Incremental
#[derive(Debug, Clone, Copy, PartialEq, Eq, crate::types::Enum)]
#[diesel(sql_type = crate::sql_types::Integer)]
#[non_exhaustive]
#[repr(i32)]
pub enum AutoVacuumMode {
    /// Freed pages stay on the freelist and the file never shrinks (default).
    None = 0,
    /// Freed pages are reclaimed and the file truncated at every commit.
    Full = 1,
    /// Freelist bookkeeping is kept, pages are reclaimed only when
    /// `incremental_vacuum` runs.
    Incremental = 2,
}

/// The mode of a [`wal_checkpoint`](SqliteConnection::wal_checkpoint) run,
/// matching the modes of
/// [`sqlite3_wal_checkpoint_v2`](https://www.sqlite.org/c3ref/wal_checkpoint_v2.html).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WalCheckpointMode {
    /// Checkpoint what is possible without waiting on readers or writers.
    Passive,
    /// Wait until there is no writer and every reader reads from the most
    /// recent snapshot, then checkpoint every frame.
    Full,
    /// Like [`Full`](Self::Full), then wait until no reader uses the WAL, so
    /// the next writer restarts the log.
    Restart,
    /// Like [`Restart`](Self::Restart), then truncate the WAL file to zero
    /// bytes.
    Truncate,
    /// Report the WAL state without checkpointing anything.
    ///
    /// Requires SQLite 3.51.0 or later. Older versions do not know this
    /// mode and silently run a [`Passive`](Self::Passive) checkpoint
    /// instead.
    Noop,
}

/// The result of a [`wal_checkpoint`](SqliteConnection::wal_checkpoint) run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WalCheckpointOutcome {
    /// Whether a busy reader or writer stopped the checkpoint early. Only
    /// the blocking modes set it: [`Passive`](WalCheckpointMode::Passive)
    /// reports `false` even when it left frames behind.
    pub busy: bool,
    /// Frames in the WAL after the checkpoint, `None` when the database is
    /// not in WAL mode.
    pub log_frames: Option<i64>,
    /// Frames of the WAL moved into the database file, `None` when the
    /// database is not in WAL mode. Counted within the current log, so a
    /// [`Truncate`](WalCheckpointMode::Truncate) run reports `Some(0)`
    /// because the log was emptied, not because nothing was moved.
    pub checkpointed_frames: Option<i64>,
}

impl SqliteConnection {
    /// Read the [`auto_vacuum`](AutoVacuumMode) mode of a database.
    ///
    /// `schema` selects an attached database by name, `None` reads `main`.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// use diesel::sqlite::AutoVacuumMode;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// assert_eq!(conn.auto_vacuum(None)?, AutoVacuumMode::None);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn auto_vacuum(&mut self, schema: Option<&str>) -> QueryResult<AutoVacuumMode> {
        use crate::query_dsl::RunQueryDsl;
        let query: Pragma<'_, crate::sql_types::Integer> = Pragma::new("auto_vacuum", schema);
        query.get_result(self)
    }

    /// Set the [`auto_vacuum`](AutoVacuumMode) mode of a database.
    ///
    /// `schema` selects an attached database by name, `None` targets `main`.
    /// Changing from or to [`AutoVacuumMode::None`] only takes effect on a
    /// database with no tables yet, or after a subsequent `VACUUM`.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// use diesel::sqlite::AutoVacuumMode;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)?;
    /// assert_eq!(conn.auto_vacuum(None)?, AutoVacuumMode::Incremental);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_auto_vacuum(
        &mut self,
        schema: Option<&str>,
        mode: AutoVacuumMode,
    ) -> QueryResult<()> {
        use crate::query_dsl::RunQueryDsl;
        // #[repr(i32)] guarantees the discriminant fits exactly in i32.
        SetPragmaInt {
            schema,
            name: "auto_vacuum",
            value: mode as i32,
        }
        .execute(self)
        .map(|_| ())
    }

    /// Total number of pages in a database, via `PRAGMA page_count`.
    ///
    /// `schema` selects an attached database by name, `None` reads `main`.
    /// Multiply by the page size for the size the database accounts for.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// use diesel::connection::SimpleConnection;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// // An empty database occupies no pages until something is written.
    /// assert_eq!(conn.page_count(None)?, 0);
    /// conn.batch_execute("CREATE TABLE items (id INTEGER PRIMARY KEY)")?;
    /// assert!(conn.page_count(None)? > 0);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn page_count(&mut self, schema: Option<&str>) -> QueryResult<i64> {
        self.read_pragma_count("page_count", schema)
    }

    /// Unused pages on a database's freelist, via `PRAGMA freelist_count`.
    ///
    /// `schema` selects an attached database by name, `None` reads `main`. A
    /// growing freelist is reclaimable space, freed by `VACUUM`.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// assert_eq!(conn.freelist_count(None)?, 0);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn freelist_count(&mut self, schema: Option<&str>) -> QueryResult<i64> {
        self.read_pragma_count("freelist_count", schema)
    }

    fn read_pragma_count(
        &mut self,
        pragma: &'static str,
        schema: Option<&str>,
    ) -> QueryResult<i64> {
        use crate::query_dsl::RunQueryDsl;

        let query: Pragma<'_, crate::sql_types::BigInt> = Pragma::new(pragma, schema);
        query.get_result(self)
    }

    /// Shrink a database by releasing freelist pages, without the full rewrite
    /// [`VACUUM`](https://www.sqlite.org/lang_vacuum.html) performs.
    ///
    /// `schema` selects an attached database by name, `None` targets `main`. `pages`
    /// bounds how many pages are reclaimed. As SQLite specifies, `None` or a value
    /// below one clears the whole freelist, as does a bound larger than it. Only
    /// databases in [`AutoVacuumMode::Incremental`] have anything to reclaim, on any
    /// other mode this succeeds and does nothing.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// use diesel::sqlite::AutoVacuumMode;
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)?;
    /// // Reclaim at most 8 pages, then whatever is left.
    /// conn.incremental_vacuum(None, Some(8))?;
    /// conn.incremental_vacuum(None, None)?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn incremental_vacuum(
        &mut self,
        schema: Option<&str>,
        pages: Option<u32>,
    ) -> QueryResult<()> {
        use crate::connection::SimpleConnection;
        use crate::query_builder::QueryBuilder;
        use crate::sqlite::SqliteQueryBuilder;

        // SQLite frees one page per step of this statement, so it only empties the
        // freelist when run to completion. `batch_execute` uses `sqlite3_exec`, which
        // does that. A prepared statement would not: `StatementUse::run` steps once,
        // which frees a single page and silently leaves the rest.
        let mut query = SqliteQueryBuilder::new();
        query.push_sql("PRAGMA ");
        query.push_identifier(schema.unwrap_or("main"))?;
        query.push_sql(".incremental_vacuum");
        if let Some(pages) = pages {
            query.push_sql("(");
            query.push_sql(&pages.to_string());
            query.push_sql(")");
        }
        self.batch_execute(&query.finish())
    }

    /// Rebuild a database, repacking it into the smallest space it can occupy.
    ///
    /// `schema` selects an attached database by name, `None` targets `main`.
    ///
    /// This cannot run inside a transaction, needs free space of up to twice the size
    /// of the database while it runs, and renumbers the implicit `rowid` of any table
    /// declared without an `INTEGER PRIMARY KEY`.
    ///
    /// Naming a schema requires SQLite 3.24.0 or later, otherwise returns an error.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.vacuum(None)?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn vacuum(&mut self, schema: Option<&str>) -> QueryResult<()> {
        use crate::query_dsl::RunQueryDsl;

        Vacuum { schema, into: None }.execute(self).map(|_| ())
    }

    /// Write a vacuumed copy of a database to `path`, leaving the original untouched.
    ///
    /// This is SQLite's online backup: the copy is consistent, defragmented, and taken
    /// without blocking readers. `schema` selects an attached database by name, `None`
    /// copies `main`. The path is a bind parameter, so it needs no quoting.
    ///
    /// `path` may name a file that does not exist or one that is empty, but writing
    /// over an existing database fails rather than replacing it.
    ///
    /// Requires SQLite 3.27.0 or later, otherwise returns an error.
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// # fn run_test() -> QueryResult<()> {
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let backup = dir.path().join("backup.db");
    /// let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.vacuum_into(None, backup.to_str().unwrap())?;
    /// # assert!(backup.exists());
    /// #     Ok(())
    /// # }
    /// ```
    pub fn vacuum_into(&mut self, schema: Option<&str>, path: &str) -> QueryResult<()> {
        use crate::query_dsl::RunQueryDsl;

        Vacuum {
            schema,
            into: Some(path),
        }
        .execute(self)
        .map(|_| ())
    }

    /// Checkpoint the [write-ahead log](https://www.sqlite.org/wal.html),
    /// moving committed frames from the WAL file into the database file.
    ///
    /// `schema` selects one attached database by name. Unlike the other
    /// maintenance helpers, `None` does not mean `main`: SQLite defines the
    /// unqualified pragma to checkpoint every attached database. With
    /// `None` and several attached databases the C API leaves the frame
    /// counts undefined.
    ///
    /// A checkpoint stopped early by a reader or writer on another
    /// connection is not an error: it sets
    /// [`busy`](WalCheckpointOutcome::busy). On a database that is not in
    /// WAL mode the call succeeds with both frame counts `None`, so it is
    /// safe to issue unconditionally. Inside a transaction on its own
    /// connection it fails with `SQLITE_LOCKED`.
    ///
    /// The mode argument requires SQLite 3.7.6 or later,
    /// [`Truncate`](WalCheckpointMode::Truncate) requires 3.8.8 or later,
    /// and [`Noop`](WalCheckpointMode::Noop) requires 3.51.0 or later.
    /// Older versions do not report an error and treat an unrecognized
    /// mode as [`Passive`](WalCheckpointMode::Passive).
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// use diesel::connection::SimpleConnection;
    /// use diesel::sqlite::WalCheckpointMode;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let path = dir.path().join("app.db");
    /// let conn = &mut SqliteConnection::establish(path.to_str().unwrap()).unwrap();
    /// conn.batch_execute("PRAGMA journal_mode = WAL")?;
    /// conn.batch_execute("CREATE TABLE logs (line TEXT NOT NULL)")?;
    ///
    /// let outcome = conn.wal_checkpoint(None, WalCheckpointMode::Truncate)?;
    /// assert!(!outcome.busy);
    /// // The whole WAL was moved into the database file and the log truncated.
    /// assert_eq!(outcome.log_frames, Some(0));
    /// assert_eq!(outcome.checkpointed_frames, Some(0));
    /// #     Ok(())
    /// # }
    /// ```
    pub fn wal_checkpoint(
        &mut self,
        schema: Option<&str>,
        mode: WalCheckpointMode,
    ) -> QueryResult<WalCheckpointOutcome> {
        use crate::query_dsl::RunQueryDsl;

        let (busy, log_frames, checkpointed_frames) =
            WalCheckpoint { schema, mode }.get_result::<(i32, i64, i64)>(self)?;
        Ok(WalCheckpointOutcome {
            busy: busy != 0,
            // On a database not in WAL mode both counts come back as -1.
            log_frames: (log_frames >= 0).then_some(log_frames),
            checkpointed_frames: (checkpointed_frames >= 0).then_some(checkpointed_frames),
        })
    }
}

// A `PRAGMA` accepts no bind parameters, neither for the schema it targets nor for the
// value it assigns, so the schema is rendered as a quoted identifier by the query
// builder. `name` is always a constant chosen here, never caller data.
struct Pragma<'a, ST> {
    schema: Option<&'a str>,
    name: &'static str,
    sql_type: PhantomData<ST>,
}

impl<'a, ST> Pragma<'a, ST> {
    fn new(name: &'static str, schema: Option<&'a str>) -> Self {
        Pragma {
            schema,
            name,
            sql_type: PhantomData,
        }
    }
}

impl<ST> QueryFragment<Sqlite> for Pragma<'_, ST> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("PRAGMA ");
        out.push_identifier(self.schema.unwrap_or("main"))?;
        out.push_sql(".");
        out.push_sql(self.name);
        Ok(())
    }
}

// The schema name is runtime data, so the rendered SQL is not determined by the type.
impl<ST> QueryId for Pragma<'_, ST> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<ST> Query for Pragma<'_, ST> {
    type SqlType = ST;
}

impl<ST> RunQueryDslSupport for Pragma<'_, ST> {}

// `PRAGMA name = value` takes no bind parameter for the value either, so the integer is
// rendered as a literal.
struct SetPragmaInt<'a> {
    schema: Option<&'a str>,
    name: &'static str,
    value: i32,
}

impl QueryFragment<Sqlite> for SetPragmaInt<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("PRAGMA ");
        out.push_identifier(self.schema.unwrap_or("main"))?;
        out.push_sql(".");
        out.push_sql(self.name);
        out.push_sql(" = ");
        out.push_sql(&self.value.to_string());
        Ok(())
    }
}

impl QueryId for SetPragmaInt<'_> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl RunQueryDslSupport for SetPragmaInt<'_> {}

// `VACUUM` names its schema as an identifier, so that operand is quoted by the query
// builder, while the `INTO` destination is an expression and binds normally.
struct Vacuum<'a> {
    schema: Option<&'a str>,
    into: Option<&'a str>,
}

impl QueryFragment<Sqlite> for Vacuum<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("VACUUM ");
        out.push_identifier(self.schema.unwrap_or("main"))?;
        if let Some(into) = self.into {
            out.push_sql(" INTO ");
            out.push_bind_param::<crate::sql_types::Text, _>(into)?;
        }
        Ok(())
    }
}

// The schema name is runtime data, so the rendered SQL is not determined by the type.
impl QueryId for Vacuum<'_> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl RunQueryDslSupport for Vacuum<'_> {}

// Like `Pragma`, no operand can be a bind parameter. Unlike `Pragma`, a
// `None` schema stays unqualified on purpose: the unqualified pragma
// checkpoints every attached database, while a qualified one targets a
// single schema. The whole checkpoint runs on the first step of the
// statement and yields exactly one row, so a prepared statement works here
// (unlike `incremental_vacuum`).
struct WalCheckpoint<'a> {
    schema: Option<&'a str>,
    mode: WalCheckpointMode,
}

impl QueryFragment<Sqlite> for WalCheckpoint<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("PRAGMA ");
        if let Some(schema) = self.schema {
            out.push_identifier(schema)?;
            out.push_sql(".");
        }
        out.push_sql(match self.mode {
            WalCheckpointMode::Passive => "wal_checkpoint(PASSIVE)",
            WalCheckpointMode::Full => "wal_checkpoint(FULL)",
            WalCheckpointMode::Restart => "wal_checkpoint(RESTART)",
            WalCheckpointMode::Truncate => "wal_checkpoint(TRUNCATE)",
            WalCheckpointMode::Noop => "wal_checkpoint(NOOP)",
        });
        Ok(())
    }
}

// The schema name and mode are runtime data, so the rendered SQL is not determined by the type.
impl QueryId for WalCheckpoint<'_> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

impl Query for WalCheckpoint<'_> {
    type SqlType = (
        crate::sql_types::Integer,
        crate::sql_types::BigInt,
        crate::sql_types::BigInt,
    );
}

impl RunQueryDslSupport for WalCheckpoint<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::SimpleConnection;
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    use crate::dsl::sql;
    use crate::prelude::*;
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    use crate::sql_types::{Integer, Text};

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    #[diesel_test_helper::test]
    fn auto_vacuum_all_modes_roundtrip_on_fresh_database() {
        for mode in [
            AutoVacuumMode::None,
            AutoVacuumMode::Full,
            AutoVacuumMode::Incremental,
        ] {
            let conn = &mut connection();
            conn.set_auto_vacuum(None, mode).unwrap();
            assert_eq!(mode, conn.auto_vacuum(None).unwrap());
        }
    }

    #[diesel_test_helper::test]
    fn auto_vacuum_incremental_sticks_across_schema_creation() {
        let conn = &mut connection();
        conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)
            .unwrap();
        assert_eq!(AutoVacuumMode::Incremental, conn.auto_vacuum(None).unwrap());

        crate::sql_query("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        assert_eq!(
            AutoVacuumMode::Incremental,
            conn.auto_vacuum(None).unwrap(),
            "the mode survives once the schema exists"
        );
    }

    #[diesel_test_helper::test]
    fn auto_vacuum_change_from_none_requires_vacuum_on_populated_database() {
        let conn = &mut connection();
        crate::sql_query("CREATE TABLE t (id INTEGER PRIMARY KEY)")
            .execute(conn)
            .unwrap();
        crate::sql_query("INSERT INTO t (id) VALUES (1)")
            .execute(conn)
            .unwrap();
        assert_eq!(AutoVacuumMode::None, conn.auto_vacuum(None).unwrap());

        // On a populated database the switch away from `None` is silently
        // deferred until a full rewrite.
        conn.set_auto_vacuum(None, AutoVacuumMode::Full).unwrap();
        assert_eq!(
            AutoVacuumMode::None,
            conn.auto_vacuum(None).unwrap(),
            "the change does not take effect without a VACUUM"
        );

        crate::sql_query("VACUUM").execute(conn).unwrap();
        assert_eq!(
            AutoVacuumMode::Full,
            conn.auto_vacuum(None).unwrap(),
            "VACUUM rewrites the file and applies the mode"
        );
    }

    #[diesel_test_helper::test]
    fn auto_vacuum_targets_the_named_attached_database() {
        let conn = &mut connection();
        crate::sql_query("ATTACH DATABASE ':memory:' AS aux")
            .execute(conn)
            .unwrap();

        conn.set_auto_vacuum(Some("aux"), AutoVacuumMode::Full)
            .unwrap();
        assert_eq!(AutoVacuumMode::Full, conn.auto_vacuum(Some("aux")).unwrap());
        assert_eq!(
            AutoVacuumMode::None,
            conn.auto_vacuum(None).unwrap(),
            "main keeps its own default"
        );
    }

    #[diesel_test_helper::test]
    fn auto_vacuum_schema_name_with_double_quote_is_handled() {
        let conn = &mut connection();
        let schema = r#"we"ird"#;
        crate::sql_query(alloc::format!(
            r#"ATTACH DATABASE ':memory:' AS "{}""#,
            schema.replace('"', "\"\"")
        ))
        .execute(conn)
        .unwrap();

        conn.set_auto_vacuum(Some(schema), AutoVacuumMode::Incremental)
            .unwrap();
        assert_eq!(
            AutoVacuumMode::Incremental,
            conn.auto_vacuum(Some(schema)).unwrap()
        );
    }

    table! {
        pragma_probe (id) {
            id -> Integer,
            payload -> Text,
        }
    }

    table! {
        aux.aux_pragma_probe (id) {
            id -> Integer,
            payload -> Text,
        }
    }

    const PROBE_TABLE: &str =
        "CREATE TABLE pragma_probe (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)";

    const AUX_PROBE_TABLE: &str =
        "CREATE TABLE aux.aux_pragma_probe (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)";

    // Large enough to spill onto overflow pages, so the database outgrows a single page
    // and leaves reclaimable pages behind once the row is deleted.
    fn overflowing_payload() -> String {
        "x".repeat(64 * 1024)
    }

    fn insert_overflowing_row(conn: &mut SqliteConnection) {
        crate::insert_into(pragma_probe::table)
            .values((
                pragma_probe::id.eq(1),
                pragma_probe::payload.eq(overflowing_payload()),
            ))
            .execute(conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn page_count_is_positive_and_grows() {
        let conn = &mut connection();
        conn.batch_execute(PROBE_TABLE).unwrap();
        let initial = conn.page_count(None).unwrap();
        assert!(initial > 0, "an initialized database has at least one page");

        insert_overflowing_row(conn);

        assert!(
            conn.page_count(None).unwrap() > initial,
            "a row spanning overflow pages grows the page count"
        );
    }

    #[diesel_test_helper::test]
    fn freelist_count_tracks_reclaimable_space() {
        let conn = &mut connection();
        assert_eq!(
            0,
            conn.freelist_count(None).unwrap(),
            "a fresh database has an empty freelist"
        );

        conn.batch_execute(PROBE_TABLE).unwrap();
        insert_overflowing_row(conn);

        crate::delete(pragma_probe::table).execute(conn).unwrap();
        assert!(
            conn.freelist_count(None).unwrap() > 0,
            "deleting the row leaves reclaimable pages on the freelist"
        );

        // `VACUUM` has no query DSL equivalent.
        crate::sql_query("VACUUM").execute(conn).unwrap();
        assert_eq!(
            0,
            conn.freelist_count(None).unwrap(),
            "VACUUM reclaims the freelist"
        );
    }

    #[diesel_test_helper::test]
    fn schema_targets_the_named_attached_database() {
        let conn = &mut connection();
        conn.batch_execute(PROBE_TABLE).unwrap();
        conn.attach_database(":memory:", "aux").unwrap();
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        crate::insert_into(aux_pragma_probe::table)
            .values((
                aux_pragma_probe::id.eq(1),
                aux_pragma_probe::payload.eq(overflowing_payload()),
            ))
            .execute(conn)
            .unwrap();

        let main_pages = conn.page_count(None).unwrap();
        let aux_pages = conn.page_count(Some("aux")).unwrap();
        assert!(
            aux_pages > main_pages,
            "the attached database holds the data, main stays small"
        );
        assert_eq!(
            main_pages,
            conn.page_count(Some("main")).unwrap(),
            "an explicit main matches the default"
        );
    }

    #[diesel_test_helper::test]
    fn schema_name_with_backtick_is_escaped() {
        // The query builder quotes SQLite identifiers with backticks, so a backtick is
        // the character that has to be doubled.
        let conn = &mut connection();
        let schema = "back`tick";
        conn.attach_database(":memory:", schema).unwrap();
        conn.batch_execute("CREATE TABLE `back``tick`.probe (id INTEGER PRIMARY KEY)")
            .unwrap();

        assert!(conn.page_count(Some(schema)).unwrap() > 0);
        assert_eq!(0, conn.freelist_count(Some(schema)).unwrap());
    }

    #[diesel_test_helper::test]
    fn unknown_schema_is_reported_as_an_error() {
        let conn = &mut connection();

        assert!(conn.page_count(Some("nope")).is_err());
        assert!(conn.freelist_count(Some("nope")).is_err());
    }

    // Leaves many pages on the freelist, so `incremental_vacuum` has something to
    // reclaim and a bound smaller than the freelist is meaningful.
    fn grow_then_empty_freelist(conn: &mut SqliteConnection) {
        conn.batch_execute(PROBE_TABLE).unwrap();
        let rows = (1..=200)
            .map(|id| {
                (
                    pragma_probe::id.eq(id),
                    pragma_probe::payload.eq("x".repeat(4000)),
                )
            })
            .collect::<Vec<_>>();
        crate::insert_into(pragma_probe::table)
            .values(rows)
            .execute(conn)
            .unwrap();
        crate::delete(pragma_probe::table).execute(conn).unwrap();
    }

    // The same, in an attached schema.
    fn grow_then_empty_aux_freelist(conn: &mut SqliteConnection) {
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        let rows = (1..=200)
            .map(|id| {
                (
                    aux_pragma_probe::id.eq(id),
                    aux_pragma_probe::payload.eq("x".repeat(4000)),
                )
            })
            .collect::<Vec<_>>();
        crate::insert_into(aux_pragma_probe::table)
            .values(rows)
            .execute(conn)
            .unwrap();
        crate::delete(aux_pragma_probe::table)
            .execute(conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_clears_the_whole_freelist() {
        let conn = &mut connection();
        conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)
            .unwrap();
        grow_then_empty_freelist(conn);
        assert!(
            conn.freelist_count(None).unwrap() > 1,
            "the deleted rows should leave many pages on the freelist"
        );

        conn.incremental_vacuum(None, None).unwrap();

        // Stepping the pragma only once would free a single page and leave the rest, so
        // this also pins that the statement is driven to completion.
        assert_eq!(0, conn.freelist_count(None).unwrap());
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_reclaims_at_most_the_requested_pages() {
        let conn = &mut connection();
        conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)
            .unwrap();
        grow_then_empty_freelist(conn);
        let before = conn.freelist_count(None).unwrap();
        assert!(before > 10, "the bound has to be smaller than the freelist");

        conn.incremental_vacuum(None, Some(10)).unwrap();

        let after = conn.freelist_count(None).unwrap();
        assert!(after >= before - 10, "at most ten pages may be reclaimed");
        assert!(after < before, "some pages should have been reclaimed");
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_is_a_no_op_outside_incremental_mode() {
        let conn = &mut connection();
        assert_eq!(AutoVacuumMode::None, conn.auto_vacuum(None).unwrap());
        grow_then_empty_freelist(conn);
        let before = conn.freelist_count(None).unwrap();
        assert!(before > 0);

        conn.incremental_vacuum(None, None).unwrap();

        assert_eq!(
            before,
            conn.freelist_count(None).unwrap(),
            "a database that is not in incremental mode keeps its freelist"
        );
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_targets_the_named_attached_database() {
        let conn = &mut connection();
        conn.attach_database(":memory:", "aux").unwrap();
        conn.set_auto_vacuum(Some("aux"), AutoVacuumMode::Incremental)
            .unwrap();

        grow_then_empty_aux_freelist(conn);
        assert!(conn.freelist_count(Some("aux")).unwrap() > 0);

        conn.incremental_vacuum(Some("aux"), None).unwrap();

        assert_eq!(0, conn.freelist_count(Some("aux")).unwrap());
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_escapes_a_backtick_in_the_schema_name() {
        // An unquoted identifier would be a syntax error, and the wrong quoting would
        // address a different database.
        let conn = &mut connection();
        let schema = "back`tick";
        conn.attach_database(":memory:", schema).unwrap();

        conn.incremental_vacuum(Some(schema), None).unwrap();

        assert_eq!(0, conn.freelist_count(Some(schema)).unwrap());
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_of_zero_pages_clears_everything() {
        // SQLite specifies that a bound below one clears the whole freelist.
        let conn = &mut connection();
        conn.set_auto_vacuum(None, AutoVacuumMode::Incremental)
            .unwrap();
        grow_then_empty_freelist(conn);
        assert!(conn.freelist_count(None).unwrap() > 0);

        conn.incremental_vacuum(None, Some(0)).unwrap();

        assert_eq!(0, conn.freelist_count(None).unwrap());
    }

    #[diesel_test_helper::test]
    fn incremental_vacuum_of_an_unknown_schema_is_an_error() {
        let conn = &mut connection();

        assert!(conn.incremental_vacuum(Some("nope"), None).is_err());
    }

    // Leaves the database holding one small row but occupying many pages, so a rebuild
    // has something to reclaim.
    fn fill_then_delete(conn: &mut SqliteConnection) {
        conn.batch_execute(PROBE_TABLE).unwrap();
        crate::insert_into(pragma_probe::table)
            .values((
                pragma_probe::id.eq(1),
                pragma_probe::payload.eq("x".repeat(256 * 1024)),
            ))
            .execute(conn)
            .unwrap();
        crate::delete(pragma_probe::table).execute(conn).unwrap();
        crate::insert_into(pragma_probe::table)
            .values((pragma_probe::id.eq(2), pragma_probe::payload.eq("kept")))
            .execute(conn)
            .unwrap();
    }

    // The same, in an attached schema.
    fn fill_then_delete_aux(conn: &mut SqliteConnection) {
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        crate::insert_into(aux_pragma_probe::table)
            .values((
                aux_pragma_probe::id.eq(1),
                aux_pragma_probe::payload.eq("x".repeat(256 * 1024)),
            ))
            .execute(conn)
            .unwrap();
        crate::delete(aux_pragma_probe::table)
            .execute(conn)
            .unwrap();
    }

    #[diesel_test_helper::test]
    fn vacuum_repacks_the_database() {
        let conn = &mut connection();
        fill_then_delete(conn);
        let before = conn.page_count(None).unwrap();
        assert!(before > 1);

        conn.vacuum(None).unwrap();

        assert!(
            conn.page_count(None).unwrap() < before,
            "rebuilding should release the pages the deleted row occupied"
        );
        assert_eq!(
            1,
            pragma_probe::table.count().get_result::<i64>(conn).unwrap(),
            "the surviving row is still there"
        );
    }

    #[diesel_test_helper::test]
    fn vacuum_targets_the_named_attached_database() {
        let conn = &mut connection();
        conn.attach_database(":memory:", "aux").unwrap();
        fill_then_delete_aux(conn);
        let before = conn.page_count(Some("aux")).unwrap();
        assert!(before > 1);

        conn.vacuum(Some("aux")).unwrap();

        assert!(conn.page_count(Some("aux")).unwrap() < before);
    }

    #[diesel_test_helper::test]
    fn vacuum_inside_a_transaction_is_an_error() {
        use crate::connection::Connection;

        let conn = &mut connection();
        let result: QueryResult<()> = conn.transaction(|conn| conn.vacuum(None));

        assert!(result.is_err());
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn vacuum_into_writes_a_readable_copy_through_a_quoted_path() {
        let dir = tempfile::tempdir().unwrap();
        // A quote in the path would break a hand-assembled statement. It is a bind
        // parameter, so it is taken verbatim.
        let destination = dir.path().join("o'brien backup.db");

        let conn = &mut connection();
        conn.batch_execute(PROBE_TABLE).unwrap();
        crate::insert_into(pragma_probe::table)
            .values((pragma_probe::id.eq(1), pragma_probe::payload.eq("copied")))
            .execute(conn)
            .unwrap();

        conn.vacuum_into(None, destination.to_str().unwrap())
            .unwrap();

        let copy = &mut SqliteConnection::establish(destination.to_str().unwrap()).unwrap();
        assert_eq!(
            "copied",
            pragma_probe::table
                .select(pragma_probe::payload)
                .get_result::<String>(copy)
                .unwrap()
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn vacuum_into_refuses_to_overwrite_an_existing_database() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("occupied.db");
        {
            let occupied = &mut SqliteConnection::establish(destination.to_str().unwrap()).unwrap();
            occupied.batch_execute(PROBE_TABLE).unwrap();
        }

        let conn = &mut connection();
        conn.batch_execute(PROBE_TABLE).unwrap();

        assert!(
            conn.vacuum_into(None, destination.to_str().unwrap())
                .is_err()
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn vacuum_into_copies_the_named_attached_database() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("aux copy.db");

        let conn = &mut connection();
        conn.attach_database(":memory:", "aux").unwrap();
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        crate::insert_into(aux_pragma_probe::table)
            .values((
                aux_pragma_probe::id.eq(7),
                aux_pragma_probe::payload.eq("copied"),
            ))
            .execute(conn)
            .unwrap();

        conn.vacuum_into(Some("aux"), destination.to_str().unwrap())
            .unwrap();

        let copy = &mut SqliteConnection::establish(destination.to_str().unwrap()).unwrap();
        // In the copy the table sits in `main`, while `aux_pragma_probe` is declared
        // schema-qualified, so this one read cannot go through it.
        let id = sql::<Integer>("SELECT id FROM aux_pragma_probe")
            .get_result::<i32>(copy)
            .unwrap();
        assert_eq!(7, id);
    }

    #[diesel_test_helper::test]
    fn vacuum_escapes_a_backtick_in_the_schema_name() {
        let conn = &mut connection();
        let schema = "back`tick";
        conn.attach_database(":memory:", schema).unwrap();

        conn.vacuum(Some(schema)).unwrap();
    }

    #[diesel_test_helper::test]
    fn vacuuming_two_schemas_rebuilds_each_of_them() {
        // The schema is part of the rendered SQL, so the two calls must not share a
        // prepared statement. If they did, the second would rebuild the first's
        // database again and leave this one untouched.
        let conn = &mut connection();
        fill_then_delete(conn);
        conn.attach_database(":memory:", "aux").unwrap();
        fill_then_delete_aux(conn);

        let main_before = conn.page_count(None).unwrap();
        let aux_before = conn.page_count(Some("aux")).unwrap();

        conn.vacuum(None).unwrap();
        conn.vacuum(Some("aux")).unwrap();

        assert!(
            conn.page_count(None).unwrap() < main_before,
            "main was rebuilt"
        );
        assert!(
            conn.page_count(Some("aux")).unwrap() < aux_before,
            "aux was rebuilt too, not main a second time"
        );
    }

    // WAL requires a real file.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn wal_connection(path: &std::path::Path) -> SqliteConnection {
        let mut conn = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        conn.batch_execute("PRAGMA journal_mode = WAL").unwrap();
        conn
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_truncate_reports_an_emptied_wal() {
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut wal_connection(&dir.path().join("wal.db"));
        conn.batch_execute(PROBE_TABLE).unwrap();
        insert_overflowing_row(conn);

        let outcome = conn
            .wal_checkpoint(None, WalCheckpointMode::Truncate)
            .unwrap();

        assert!(!outcome.busy);
        assert_eq!(Some(0), outcome.log_frames, "the WAL file was truncated");
        assert_eq!(Some(0), outcome.checkpointed_frames);
    }

    #[diesel_test_helper::test]
    fn wal_checkpoint_outside_wal_mode_reports_no_frames() {
        let conn = &mut connection();

        let outcome = conn
            .wal_checkpoint(None, WalCheckpointMode::Truncate)
            .unwrap();

        assert!(!outcome.busy);
        assert_eq!(None, outcome.log_frames);
        assert_eq!(None, outcome.checkpointed_frames);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_accepts_every_mode() {
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut wal_connection(&dir.path().join("modes.db"));
        conn.batch_execute(PROBE_TABLE).unwrap();

        for (row, mode) in [
            WalCheckpointMode::Passive,
            WalCheckpointMode::Full,
            WalCheckpointMode::Restart,
            WalCheckpointMode::Truncate,
            WalCheckpointMode::Noop,
        ]
        .into_iter()
        .enumerate()
        {
            // A fresh row per round gives every mode frames to move.
            crate::insert_into(pragma_probe::table)
                .values((
                    pragma_probe::id.eq(i32::try_from(row).unwrap() + 1),
                    pragma_probe::payload.eq("row"),
                ))
                .execute(conn)
                .unwrap();

            let outcome = conn.wal_checkpoint(None, mode).unwrap();
            assert!(!outcome.busy, "{mode:?} had no competing readers");
            assert!(
                outcome.log_frames.is_some(),
                "{mode:?} ran on a WAL database"
            );
            assert!(outcome.checkpointed_frames.is_some());
            assert!(
                outcome.checkpointed_frames <= outcome.log_frames,
                "{mode:?}: checkpointed frames cannot exceed the log size"
            );
        }
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_noop_reports_state_without_moving_frames() {
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut wal_connection(&dir.path().join("noop.db"));

        // NOOP exists since SQLite 3.51.0, older versions run PASSIVE instead.
        let version = crate::select(sql::<Text>("sqlite_version()"))
            .get_result::<String>(conn)
            .unwrap();
        let mut parts = version.split('.').map(|part| part.parse::<u32>().unwrap());
        if (parts.next().unwrap(), parts.next().unwrap()) < (3, 51) {
            return;
        }

        conn.batch_execute(PROBE_TABLE).unwrap();
        conn.wal_checkpoint(None, WalCheckpointMode::Truncate)
            .unwrap();
        crate::insert_into(pragma_probe::table)
            .values((pragma_probe::id.eq(1), pragma_probe::payload.eq("noop")))
            .execute(conn)
            .unwrap();

        let first = conn.wal_checkpoint(None, WalCheckpointMode::Noop).unwrap();
        let second = conn.wal_checkpoint(None, WalCheckpointMode::Noop).unwrap();

        assert!(!first.busy, "NOOP never blocks");
        assert!(first.log_frames > Some(0), "the insert sits in the WAL");
        assert_eq!(Some(0), first.checkpointed_frames, "nothing was moved");
        assert_eq!(first, second, "a second NOOP reports the same state");
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_reports_busy_while_a_reader_holds_an_old_snapshot() {
        use crate::connection::Connection;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("busy.db");
        let writer = &mut wal_connection(&path);
        writer.batch_execute(PROBE_TABLE).unwrap();
        insert_overflowing_row(writer);

        let reader = &mut SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        reader
            .transaction::<_, crate::result::Error, _>(|reader| {
                // Take the read snapshot, BEGIN alone defers it to the first read.
                let _ = pragma_probe::table.count().get_result::<i64>(reader)?;

                // Grow the WAL past the reader's snapshot, so a blocking
                // checkpoint cannot complete.
                crate::insert_into(pragma_probe::table)
                    .values((pragma_probe::id.eq(2), pragma_probe::payload.eq("late")))
                    .execute(writer)?;

                // Passive is never reported busy, it checkpoints up to the
                // reader's snapshot and leaves the rest.
                let outcome = writer.wal_checkpoint(None, WalCheckpointMode::Passive)?;
                assert!(!outcome.busy, "PASSIVE never reports busy");
                assert!(
                    outcome.checkpointed_frames < outcome.log_frames,
                    "the frames past the reader's snapshot stay in the WAL"
                );

                for mode in [
                    WalCheckpointMode::Full,
                    WalCheckpointMode::Restart,
                    WalCheckpointMode::Truncate,
                ] {
                    let outcome = writer.wal_checkpoint(None, mode)?;
                    assert!(outcome.busy, "the open reader blocks a {mode:?} checkpoint");
                }
                Ok(())
            })
            .unwrap();

        let outcome = writer
            .wal_checkpoint(None, WalCheckpointMode::Truncate)
            .unwrap();
        assert!(
            !outcome.busy,
            "the checkpoint completes once the reader is done"
        );
        assert_eq!(Some(0), outcome.log_frames);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_targets_the_named_attached_database() {
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut connection();
        conn.attach_database(dir.path().join("aux.db").to_str().unwrap(), "aux")
            .unwrap();
        conn.batch_execute("PRAGMA aux.journal_mode = WAL").unwrap();
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        crate::insert_into(aux_pragma_probe::table)
            .values((
                aux_pragma_probe::id.eq(1),
                aux_pragma_probe::payload.eq("row"),
            ))
            .execute(conn)
            .unwrap();

        let outcome = conn
            .wal_checkpoint(Some("aux"), WalCheckpointMode::Truncate)
            .unwrap();
        assert!(!outcome.busy);
        assert_eq!(
            Some(0),
            outcome.log_frames,
            "the attached database was checkpointed"
        );

        // `main` is not in WAL mode, so a checkpoint naming it reports no frames.
        let outcome = conn
            .wal_checkpoint(Some("main"), WalCheckpointMode::Truncate)
            .unwrap();
        assert_eq!(None, outcome.log_frames);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_unqualified_covers_every_attached_database() {
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut wal_connection(&dir.path().join("main.db"));
        conn.batch_execute(PROBE_TABLE).unwrap();
        insert_overflowing_row(conn);
        conn.attach_database(dir.path().join("aux.db").to_str().unwrap(), "aux")
            .unwrap();
        conn.batch_execute("PRAGMA aux.journal_mode = WAL").unwrap();
        conn.batch_execute(AUX_PROBE_TABLE).unwrap();
        crate::insert_into(aux_pragma_probe::table)
            .values((
                aux_pragma_probe::id.eq(1),
                aux_pragma_probe::payload.eq("row"),
            ))
            .execute(conn)
            .unwrap();

        conn.wal_checkpoint(None, WalCheckpointMode::Truncate)
            .unwrap();

        // Both WALs are empty afterwards, which a qualified passive
        // checkpoint reports without moving anything.
        let main_after = conn
            .wal_checkpoint(Some("main"), WalCheckpointMode::Passive)
            .unwrap();
        assert_eq!(Some(0), main_after.log_frames, "main was checkpointed");
        let aux_after = conn
            .wal_checkpoint(Some("aux"), WalCheckpointMode::Passive)
            .unwrap();
        assert_eq!(Some(0), aux_after.log_frames, "aux was checkpointed too");
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_escapes_a_double_quote_in_the_schema_name() {
        // An unquoted identifier would be a syntax error, and the wrong quoting
        // would address a different database.
        let dir = tempfile::tempdir().unwrap();
        let conn = &mut connection();
        let schema = r#"we"ird"#;
        let quoted = schema.replace('"', "\"\"");
        conn.attach_database(dir.path().join("weird.db").to_str().unwrap(), schema)
            .unwrap();
        conn.batch_execute(&alloc::format!(r#"PRAGMA "{quoted}".journal_mode = WAL"#))
            .unwrap();
        conn.batch_execute(&alloc::format!(
            r#"CREATE TABLE "{quoted}".t (id INTEGER PRIMARY KEY)"#
        ))
        .unwrap();

        let outcome = conn
            .wal_checkpoint(Some(schema), WalCheckpointMode::Truncate)
            .unwrap();
        assert_eq!(Some(0), outcome.log_frames, "the quoted schema was reached");
    }

    #[diesel_test_helper::test]
    fn wal_checkpoint_of_an_unknown_schema_is_an_error() {
        let conn = &mut connection();

        assert!(
            conn.wal_checkpoint(Some("nope"), WalCheckpointMode::Passive)
                .is_err()
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn wal_checkpoint_inside_a_transaction_is_an_error() {
        use crate::connection::Connection;

        let dir = tempfile::tempdir().unwrap();
        let conn = &mut wal_connection(&dir.path().join("txn.db"));
        conn.batch_execute(PROBE_TABLE).unwrap();

        let result: QueryResult<WalCheckpointOutcome> = conn.transaction(|conn| {
            crate::insert_into(pragma_probe::table)
                .values((pragma_probe::id.eq(1), pragma_probe::payload.eq("txn")))
                .execute(conn)?;
            conn.wal_checkpoint(None, WalCheckpointMode::Truncate)
        });

        assert!(result.is_err(), "SQLite reports SQLITE_LOCKED");
    }
}
