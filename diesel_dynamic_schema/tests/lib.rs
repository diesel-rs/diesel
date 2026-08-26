extern crate diesel;
extern crate diesel_dynamic_schema;

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
fn dynamic_group_by_having_filters_groups() {
    use diesel::dsl::count;

    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");

    let counts = users
        .select((name, count(id)))
        .group_by(name)
        .having(count(id).gt(1))
        .order(name)
        .load::<(String, i64)>(conn);

    assert_eq!(Ok(vec![("Sean".into(), 2)]), counts);
}

#[test]
fn dynamic_group_by_multiple_columns() {
    use diesel::dsl::count;

    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query(
        "INSERT INTO users (name, hair_color) VALUES \
         ('Sean', 'black'), ('Sean', 'black'), ('Sean', 'blonde'), ('Tess', 'red')",
    )
    .execute(conn)
    .unwrap();

    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");
    let hair_color = users.column::<Text, _>("hair_color");

    let counts = users
        .select((name, hair_color, count(id)))
        .group_by((name, hair_color))
        .order((name, hair_color))
        .load::<(String, String, i64)>(conn);

    assert_eq!(
        Ok(vec![
            ("Sean".into(), "black".into(), 2),
            ("Sean".into(), "blonde".into(), 1),
            ("Tess".into(), "red".into(), 1),
        ]),
        counts
    );
}

#[test]
fn dynamic_group_by_without_aggregate_deduplicates() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let name = users.column::<Text, _>("name");

    let names = users
        .select(name)
        .group_by(name)
        .order(name)
        .load::<String>(conn);

    assert_eq!(Ok(vec!["Sean".to_string(), "Tess".to_string()]), names);
}
