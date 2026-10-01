extern crate diesel;
extern crate diesel_dynamic_schema;

#[cfg(feature = "postgres")]
use diesel::connection::SimpleConnection;
use diesel::ddl::TableDdl;
use diesel::sql_types::*;
use diesel::*;
use diesel_dynamic_schema::{schema, table};

mod dynamic_values;

mod connection_setup;

use connection_setup::{create_user_table, establish_connection};

#[cfg(feature = "postgres")]
type Backend = diesel::pg::Pg;
#[cfg(feature = "mysql")]
type Backend = diesel::mysql::Mysql;
#[cfg(feature = "mariadb")]
type Backend = diesel::mariadb::Mariadb;
#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
type Backend = diesel::sqlite::Sqlite;

#[test]
fn querying_basic_schemas() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users(name) VALUES ('Sean')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let name = users.column::<Text, _>("name");
    let names = users.select(name).load::<String>(conn);
    assert_eq!(Ok(vec!["Sean".into()]), names);
}

#[test]
fn querying_multiple_types() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let hair_color = users.column::<Nullable<Text>, _>("hair_color");
    let name = users.column::<Text, _>("name");
    let users = users
        .select((name, hair_color))
        .load::<(String, Option<String>)>(conn);
    assert_eq!(
        Ok(vec![("Sean".into(), None), ("Tess".into(), None)]),
        users
    );
}

#[test]
fn columns_used_in_where_clause() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let name = users.column::<Text, _>("name");
    let users = users
        .select(name)
        .filter(name.eq("Sean"))
        .load::<String>(conn);

    assert_eq!(Ok(vec!["Sean".into()]), users);
}

#[test]
#[cfg(any(
    feature = "postgres",
    feature = "mysql",
    feature = "mariadb",
    feature = "sqlite",
    feature = "sqlite-no-std"
))]
fn providing_custom_schema_name() {
    let table = schema("information_schema").table("users");
    let sql = debug_query::<Backend, _>(&table);

    #[cfg(feature = "postgres")]
    assert_eq!(
        r#""information_schema"."users" -- binds: []"#,
        sql.to_string()
    );

    #[cfg(not(feature = "postgres"))]
    assert_eq!("`information_schema`.`users` -- binds: []", sql.to_string());
}

#[test]
#[cfg(any(
    feature = "postgres",
    feature = "mysql",
    feature = "mariadb",
    feature = "sqlite"
))]
fn drop_table_debug_sql_for_dynamic_table() {
    let query = table("dynamic_drop_table_debug_target").drop_table();
    let sql = debug_query::<Backend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(
            r#"DROP TABLE "dynamic_drop_table_debug_target" -- binds: []"#,
            sql
        );
    } else {
        assert_eq!(
            "DROP TABLE `dynamic_drop_table_debug_target` -- binds: []",
            sql
        );
    }
}

#[test]
#[cfg(any(
    feature = "postgres",
    feature = "mysql",
    feature = "mariadb",
    feature = "sqlite"
))]
fn drop_table_if_exists_debug_sql_for_dynamic_table() {
    let query = table("dynamic_drop_table_debug_target")
        .drop_table()
        .if_exists();
    let sql = debug_query::<Backend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(
            r#"DROP TABLE IF EXISTS "dynamic_drop_table_debug_target" -- binds: []"#,
            sql
        );
    } else {
        assert_eq!(
            "DROP TABLE IF EXISTS `dynamic_drop_table_debug_target` -- binds: []",
            sql
        );
    }
}

#[test]
#[cfg(any(
    feature = "postgres",
    feature = "mysql",
    feature = "mariadb",
    feature = "sqlite"
))]
fn drop_table_schema_qualified_debug_sql_for_dynamic_table() {
    let query = schema("dynamic_schema")
        .table("dynamic_drop_table_debug_target")
        .drop_table()
        .if_exists();
    let sql = debug_query::<Backend, _>(&query).to_string();

    if cfg!(feature = "postgres") {
        assert_eq!(
            r#"DROP TABLE IF EXISTS "dynamic_schema"."dynamic_drop_table_debug_target" -- binds: []"#,
            sql
        );
    } else {
        assert_eq!(
            "DROP TABLE IF EXISTS `dynamic_schema`.`dynamic_drop_table_debug_target` -- binds: []",
            sql
        );
    }
}

#[test]
#[cfg(feature = "postgres")]
fn drop_table_cascade_debug_sql_for_dynamic_table() {
    let query = table("dynamic_drop_table_debug_target")
        .drop_table()
        .if_exists()
        .cascade();
    let sql = debug_query::<Backend, _>(&query).to_string();

    assert_eq!(
        r#"DROP TABLE IF EXISTS "dynamic_drop_table_debug_target" CASCADE -- binds: []"#,
        sql
    );
}

#[test]
#[cfg(feature = "postgres")]
fn drop_table_cascade_removes_dependent_constraints_for_dynamic_table() {
    let conn = &mut establish_connection();
    let parent = table("dynamic_drop_table_cascade_parent");
    let child = table("dynamic_drop_table_cascade_child");

    child.drop_table().if_exists().execute(conn).unwrap();
    parent
        .drop_table()
        .if_exists()
        .cascade()
        .execute(conn)
        .unwrap();

    conn.batch_execute(
        "CREATE TABLE dynamic_drop_table_cascade_parent (id INTEGER PRIMARY KEY);\
         CREATE TABLE dynamic_drop_table_cascade_child (\
             id INTEGER PRIMARY KEY,\
             parent_id INTEGER NOT NULL REFERENCES dynamic_drop_table_cascade_parent(id)\
         )",
    )
    .unwrap();
    let rollback = conn.transaction::<(), diesel::result::Error, _>(|conn| {
        let blocked_drop = parent.drop_table().execute(conn);
        assert!(blocked_drop.is_err());
        Err(diesel::result::Error::RollbackTransaction)
    });
    assert!(matches!(
        rollback,
        Err(diesel::result::Error::RollbackTransaction)
    ));

    parent.drop_table().cascade().execute(conn).unwrap();

    let result =
        sql_query("INSERT INTO dynamic_drop_table_cascade_child (id, parent_id) VALUES (1, 1)")
            .execute(conn);
    assert_eq!(Ok(1), result);

    child.drop_table().execute(conn).unwrap();
}

#[test]
fn drop_table_drops_dynamic_table() {
    let conn = &mut establish_connection();
    let target = table("dynamic_drop_table_runtime_target");
    target.drop_table().if_exists().execute(conn).unwrap();

    #[cfg(feature = "postgres")]
    let create_table = "CREATE TABLE dynamic_drop_table_runtime_target (id INTEGER PRIMARY KEY)";
    #[cfg(feature = "sqlite")]
    let create_table = "CREATE TABLE dynamic_drop_table_runtime_target (id INTEGER PRIMARY KEY)";
    #[cfg(any(feature = "mysql", feature = "mariadb"))]
    let create_table =
        "CREATE TEMPORARY TABLE dynamic_drop_table_runtime_target (id INTEGER PRIMARY KEY)";

    sql_query(create_table).execute(conn).unwrap();
    target.drop_table().execute(conn).unwrap();

    let result =
        sql_query("INSERT INTO dynamic_drop_table_runtime_target (id) VALUES (1)").execute(conn);
    assert!(result.is_err());
}

#[test]
fn drop_table_if_exists_accepts_absent_dynamic_table() {
    let conn = &mut establish_connection();
    table("dynamic_drop_table_if_exists_target")
        .drop_table()
        .if_exists()
        .execute(conn)
        .unwrap();
}
