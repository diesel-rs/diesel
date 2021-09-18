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
#[cfg(feature = "sqlite")]
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
