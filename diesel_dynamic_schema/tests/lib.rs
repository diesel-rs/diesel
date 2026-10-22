extern crate diesel;
extern crate diesel_dynamic_schema;

use diesel::sql_types::*;
use diesel::*;
use diesel_dynamic_schema::{schema, table};

mod dynamic_values;

mod connection_setup;

use connection_setup::{create_posts_table, create_user_table, establish_connection};

mod typed {
    diesel::table! {
        users (id) {
            id -> Integer,
            name -> Text,
        }
    }

    diesel::table! {
        dynamic_schema_posts (id) {
            id -> Integer,
            user_id -> Integer,
        }
    }

    diesel::allow_tables_to_appear_in_same_query!(users, dynamic_schema_posts);
}

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
fn columns_used_in_subselect_where_clause() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    create_posts_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");
    let sean_id = users
        .select(id)
        .filter(name.eq("Sean"))
        .get_result::<i32>(conn)
        .unwrap();
    insert_into(typed::dynamic_schema_posts::table)
        .values(typed::dynamic_schema_posts::user_id.eq(sean_id))
        .execute(conn)
        .unwrap();

    // `id` names the outer `users` table from inside the subselect
    let users_with_posts = typed::users::table
        .select(typed::users::name)
        .filter(dsl::exists(
            typed::dynamic_schema_posts::table.filter(typed::dynamic_schema_posts::user_id.eq(id)),
        ))
        .load::<String>(conn);

    assert_eq!(Ok(vec!["Sean".into()]), users_with_posts);
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
