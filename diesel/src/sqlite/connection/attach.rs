use super::SqliteConnection;
use crate::query_builder::{AstPass, QueryFragment, QueryId};
use crate::query_dsl::RunQueryDslSupport;
use crate::result::QueryResult;
use crate::sqlite::Sqlite;

impl SqliteConnection {
    /// Attach the database file at `path` under `schema_name`.
    ///
    /// Runs [`ATTACH DATABASE ? AS ?`](https://www.sqlite.org/lang_attach.html) with
    /// both operands bound as parameters, so no SQL string escaping is needed.
    /// Diesel always opens connections with `SQLITE_OPEN_URI`, so a path beginning
    /// with `file:` is interpreted as a URI exactly as it is in
    /// [`SqliteConnection::establish`](crate::connection::Connection::establish).
    ///
    /// A missing file is created as an empty database unless
    /// [`set_attach_create_enabled(false)`][Self::set_attach_create_enabled] is set,
    /// and [`set_attach_write_enabled(false)`][Self::set_attach_write_enabled] attaches
    /// read-only. The attach count is bounded (10 by default). A transaction across the
    /// main and an attached file is crash-atomic per file only under WAL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    /// conn.attach_database(":memory:", "aux")?;
    /// conn.detach_database("aux")?;
    /// #     Ok(())
    /// # }
    /// ```
    pub fn attach_database(&mut self, path: &str, schema_name: &str) -> QueryResult<()> {
        use crate::query_dsl::RunQueryDsl;
        AttachDatabase { path, schema_name }
            .execute(self)
            .map(|_| ())
    }

    /// Detach the database previously attached under `schema_name`.
    ///
    /// Runs [`DETACH DATABASE ?`](https://www.sqlite.org/lang_detach.html) with the
    /// schema name bound as a parameter. Detaching a schema still in use fails with an
    /// ordinary error.
    pub fn detach_database(&mut self, schema_name: &str) -> QueryResult<()> {
        use crate::query_dsl::RunQueryDsl;
        DetachDatabase { schema_name }.execute(self).map(|_| ())
    }
}

#[derive(QueryId)]
struct AttachDatabase<'a> {
    path: &'a str,
    schema_name: &'a str,
}

impl QueryFragment<Sqlite> for AttachDatabase<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("ATTACH DATABASE ");
        out.push_bind_param::<crate::sql_types::Text, _>(self.path)?;
        out.push_sql(" AS ");
        out.push_bind_param::<crate::sql_types::Text, _>(self.schema_name)?;
        Ok(())
    }
}

impl RunQueryDslSupport for AttachDatabase<'_> {}

#[derive(QueryId)]
struct DetachDatabase<'a> {
    schema_name: &'a str,
}

impl QueryFragment<Sqlite> for DetachDatabase<'_> {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Sqlite>) -> QueryResult<()> {
        out.push_sql("DETACH DATABASE ");
        out.push_bind_param::<crate::sql_types::Text, _>(self.schema_name)?;
        Ok(())
    }
}

impl RunQueryDslSupport for DetachDatabase<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::sql;
    use crate::prelude::*;
    use crate::sql_types::Integer;

    fn connection() -> SqliteConnection {
        SqliteConnection::establish(":memory:").unwrap()
    }

    // These ATTACH tests need a real filesystem (temp files), which is not
    // available on the wasm target, where SQLite is in-memory only.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn temp_db_path(name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        (dir, path)
    }

    // Tables used by the ATTACH round-trip tests below. Their CREATE statements are
    // DDL (raw SQL), but rows and reads go through the typed query DSL.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    table! {
        attach_owners (id) {
            id -> Integer,
            name -> Text,
        }
    }
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    table! {
        aux.attach_pets (id) {
            id -> Integer,
            owner_id -> Integer,
            name -> Text,
        }
    }
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    allow_tables_to_appear_in_same_query!(attach_owners, attach_pets);
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    table! {
        attach_marker (id) {
            id -> Integer,
        }
    }
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    table! {
        ro.readonly_marker (id) {
            id -> Integer,
        }
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_database_supports_cross_schema_join_then_detach() {
        use crate::connection::SimpleConnection;
        let conn = &mut connection();

        conn.attach_database(":memory:", "aux").unwrap();

        // Schema-qualified CREATE is DDL and stays raw SQL. The rows and the join
        // below use the typed query DSL.
        conn.batch_execute(
            "CREATE TABLE attach_owners (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE aux.attach_pets (id INTEGER PRIMARY KEY, owner_id INTEGER, name TEXT NOT NULL);",
        )
        .unwrap();

        crate::insert_into(attach_owners::table)
            .values(&[
                (attach_owners::id.eq(1), attach_owners::name.eq("Sean")),
                (attach_owners::id.eq(2), attach_owners::name.eq("Tess")),
            ])
            .execute(conn)
            .unwrap();
        crate::insert_into(attach_pets::table)
            .values((
                attach_pets::id.eq(1),
                attach_pets::owner_id.eq(1),
                attach_pets::name.eq("Ferris"),
            ))
            .execute(conn)
            .unwrap();

        let pet_owner = attach_owners::table
            .inner_join(attach_pets::table.on(attach_pets::owner_id.eq(attach_owners::id)))
            .filter(attach_pets::name.eq("Ferris"))
            .select(attach_owners::name)
            .get_result::<String>(conn)
            .unwrap();
        assert_eq!(pet_owner, "Sean");

        conn.detach_database("aux").unwrap();

        // The attached schema is gone, so querying it now fails.
        assert!(
            attach_pets::table
                .select(attach_pets::name)
                .get_result::<String>(conn)
                .is_err()
        );
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_database_binds_path_verbatim_without_quoting() {
        // A single quote in the path would break a hand-assembled ATTACH statement.
        // Bound parameters take the path verbatim.
        let (_dir, path) = temp_db_path("o'brien.db");

        conn_attach_roundtrip(&path);

        // The table created through ATTACH persisted to the literal file, so a
        // fresh connection to that exact path can read it.
        let mut direct = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        let count = attach_marker::table
            .count()
            .get_result::<i64>(&mut direct)
            .unwrap();
        assert_eq!(count, 0);
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    fn conn_attach_roundtrip(path: &std::path::Path) {
        use crate::connection::SimpleConnection;
        let conn = &mut connection();
        conn.attach_database(path.to_str().unwrap(), "verbatim")
            .unwrap();
        conn.batch_execute("CREATE TABLE verbatim.attach_marker (id INTEGER PRIMARY KEY)")
            .unwrap();
        conn.detach_database("verbatim").unwrap();
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_database_interprets_file_uri_query_parameters() {
        // Seed a database with a row to read through the attached schema.
        let (_dir, path) = temp_db_path("uri_seed.db");
        {
            let mut seed = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
            crate::sql_query("CREATE TABLE t (id INTEGER PRIMARY KEY)")
                .execute(&mut seed)
                .unwrap();
            crate::sql_query("INSERT INTO t (id) VALUES (1)")
                .execute(&mut seed)
                .unwrap();
        }

        // Attach with a `file:` URI and `mode=ro`. If SQLite treated the bound
        // string as a literal filename it would fail to find the file; interpreting
        // it as a URI opens the real file in read-only mode instead.
        let uri = format!("file:{}?mode=ro", path.display());
        let conn = &mut connection();
        conn.attach_database(&uri, "ro_schema").unwrap();

        // Read from the attached schema: proves the ATTACH opened a real file.
        let id: i64 = sql::<crate::sql_types::BigInt>("SELECT id FROM ro_schema.t")
            .get_result(conn)
            .unwrap();
        assert_eq!(id, 1);

        // Write fails: the URI `mode=ro` parameter took effect.
        assert!(
            crate::sql_query("INSERT INTO ro_schema.t (id) VALUES (2)")
                .execute(conn)
                .is_err()
        );

        conn.detach_database("ro_schema").unwrap();
    }

    #[diesel_test_helper::test]
    fn attach_and_detach_surface_errors_without_panicking() {
        let conn = &mut connection();

        // A duplicate schema name on attach is an error, not a panic.
        conn.attach_database(":memory:", "dup").unwrap();
        assert!(conn.attach_database(":memory:", "dup").is_err());
        conn.detach_database("dup").unwrap();

        // Detaching an unknown schema is likewise an error. Detaching one still in
        // use cannot occur through the safe API: an in-flight iterator holds
        // `&mut conn`, so no detach can overlap it.
        assert!(conn.detach_database("never_attached").is_err());
    }

    #[diesel_test_helper::test]
    fn attach_database_binds_schema_name_verbatim_without_identifier_quoting() {
        use crate::connection::SimpleConnection;
        let conn = &mut connection();

        // A space or quote in the schema name would need identifier quoting in a
        // hand-assembled statement. Bound as a parameter it is taken verbatim. The
        // reference below stays raw SQL: such a schema is not expressible via `table!`.
        let schema = "weird 'schema";
        conn.attach_database(":memory:", schema).unwrap();

        conn.batch_execute(
            r#"CREATE TABLE "weird 'schema".t (id INTEGER PRIMARY KEY);
             INSERT INTO "weird 'schema".t (id) VALUES (7);"#,
        )
        .unwrap();
        let id = sql::<Integer>(r#"SELECT id FROM "weird 'schema".t"#)
            .get_result::<i32>(conn)
            .unwrap();
        assert_eq!(id, 7);

        conn.detach_database(schema).unwrap();
    }

    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    #[diesel_test_helper::test]
    fn attach_database_honors_create_and_write_hardening_knobs() {
        use crate::connection::SimpleConnection;
        let conn = &mut connection();

        // ATTACH_CREATE and ATTACH_WRITE need SQLite 3.49.0+, so skip on older libraries.
        if conn.set_attach_create_enabled(false).is_err() {
            return;
        }

        // Create disabled: attaching a nonexistent path fails and materializes nothing.
        let (_dir_missing, missing) = temp_db_path("nocreate.db");
        assert!(
            conn.attach_database(missing.to_str().unwrap(), "missing")
                .is_err()
        );
        assert!(!missing.exists());

        // Write disabled: an existing database attaches read-only, so writes fail.
        conn.set_attach_write_enabled(false).unwrap();
        let (_dir_existing, existing) = temp_db_path("readonly.db");
        {
            let mut seed = SqliteConnection::establish(existing.to_str().unwrap()).unwrap();
            seed.batch_execute("CREATE TABLE readonly_marker (id INTEGER)")
                .unwrap();
        }
        conn.attach_database(existing.to_str().unwrap(), "ro")
            .unwrap();
        assert!(
            crate::insert_into(readonly_marker::table)
                .values(readonly_marker::id.eq(1))
                .execute(conn)
                .is_err()
        );
        conn.detach_database("ro").unwrap();
    }
}
