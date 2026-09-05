use crate::schema::{TestBackend, connection_without_transaction};
#[cfg(feature = "postgres")]
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::{debug_query, insert_into};

table! {
    drop_table_debug_target (id) {
        id -> Integer,
    }
}

table! {
    #[sql_name = "drop_table_debug_target"]
    drop_table_debug_target_alias (id) {
        id -> Integer,
    }
}

table! {
    drop_table_runtime_target (id) {
        id -> Integer,
    }
}

table! {
    drop_table_if_exists_target (id) {
        id -> Integer,
    }
}

#[cfg(feature = "postgres")]
table! {
    custom_schema.drop_table_custom_schema (id) {
        id -> Int4,
    }
}

#[cfg(feature = "postgres")]
table! {
    drop_table_cascade_parent (id) {
        id -> Integer,
    }
}

#[cfg(feature = "postgres")]
table! {
    drop_table_cascade_child (id) {
        id -> Integer,
        parent_id -> Integer,
    }
}

#[diesel_test_helper::test]
fn drop_table_debug_sql() {
    let query = drop_table_debug_target::table.drop_table();
    let sql = debug_query::<TestBackend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(r#"DROP TABLE "drop_table_debug_target" -- binds: []"#, sql);
    } else {
        assert_eq!("DROP TABLE `drop_table_debug_target` -- binds: []", sql);
    }
}

// One value per table, so `Eq` can only be checked at the type level.
#[diesel_test_helper::test]
fn drop_table_statement_implements_eq() {
    fn assert_eq_impl<T: Eq>(_: &T) {}

    assert_eq_impl(&drop_table_debug_target::table.drop_table());
    assert_eq_impl(&drop_table_debug_target::table.drop_table().if_exists());
}

#[diesel_test_helper::test]
fn drop_table_if_exists_debug_sql() {
    let query = drop_table_debug_target::table.drop_table().if_exists();
    let sql = debug_query::<TestBackend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(
            r#"DROP TABLE IF EXISTS "drop_table_debug_target" -- binds: []"#,
            sql
        );
    } else {
        assert_eq!(
            "DROP TABLE IF EXISTS `drop_table_debug_target` -- binds: []",
            sql
        );
    }
}

#[diesel_test_helper::test]
fn drop_table_uses_sql_name() {
    let query = drop_table_debug_target_alias::table.drop_table();
    let sql = debug_query::<TestBackend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(r#"DROP TABLE "drop_table_debug_target" -- binds: []"#, sql);
    } else {
        assert_eq!("DROP TABLE `drop_table_debug_target` -- binds: []", sql);
    }
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn drop_table_schema_qualified_debug_sql() {
    let query = drop_table_custom_schema::table.drop_table().if_exists();
    let sql = debug_query::<TestBackend, _>(&query).to_string();

    assert_eq!(
        r#"DROP TABLE IF EXISTS "custom_schema"."drop_table_custom_schema" -- binds: []"#,
        sql
    );
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn drop_table_cascade_debug_sql() {
    let query = drop_table_debug_target::table
        .drop_table()
        .if_exists()
        .cascade();
    let sql = debug_query::<TestBackend, _>(&query).to_string();

    assert_eq!(
        r#"DROP TABLE IF EXISTS "drop_table_debug_target" CASCADE -- binds: []"#,
        sql
    );
}

#[diesel_test_helper::test]
fn drop_table_drops_existing_table() {
    use crate::schema_dsl::*;

    let conn = &mut connection_without_transaction();
    drop_table_runtime_target::table
        .drop_table()
        .if_exists()
        .execute(conn)
        .unwrap();
    create_table("drop_table_runtime_target", (integer("id").primary_key(),))
        .execute(conn)
        .unwrap();

    drop_table_runtime_target::table
        .drop_table()
        .execute(conn)
        .unwrap();

    let result = insert_into(drop_table_runtime_target::table)
        .values(drop_table_runtime_target::id.eq(1))
        .execute(conn);
    assert!(result.is_err());
}

#[diesel_test_helper::test]
fn drop_table_if_exists_accepts_absent_table() {
    let conn = &mut connection_without_transaction();
    drop_table_if_exists_target::table
        .drop_table()
        .if_exists()
        .execute(conn)
        .unwrap();
}

#[diesel_test_helper::test]
#[cfg(feature = "postgres")]
fn drop_table_cascade_removes_dependent_constraints() {
    let conn = &mut connection_without_transaction();
    drop_table_cascade_child::table
        .drop_table()
        .if_exists()
        .execute(conn)
        .unwrap();
    drop_table_cascade_parent::table
        .drop_table()
        .if_exists()
        .cascade()
        .execute(conn)
        .unwrap();

    // DDL for this foreign key has no typed Diesel builder yet.
    conn.batch_execute(
        "CREATE TABLE drop_table_cascade_parent (id INTEGER PRIMARY KEY);\
         CREATE TABLE drop_table_cascade_child (\
             id INTEGER PRIMARY KEY,\
             parent_id INTEGER NOT NULL REFERENCES drop_table_cascade_parent(id)\
         )",
    )
    .unwrap();
    let blocked_drop = drop_table_cascade_parent::table.drop_table().execute(conn);
    assert!(blocked_drop.is_err());

    drop_table_cascade_parent::table
        .drop_table()
        .cascade()
        .execute(conn)
        .unwrap();

    let result = insert_into(drop_table_cascade_child::table)
        .values((
            drop_table_cascade_child::id.eq(1),
            drop_table_cascade_child::parent_id.eq(1),
        ))
        .execute(conn);
    assert_eq!(Ok(1), result);

    drop_table_cascade_child::table
        .drop_table()
        .execute(conn)
        .unwrap();
}
