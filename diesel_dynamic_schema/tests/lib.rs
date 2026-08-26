extern crate diesel;
extern crate diesel_dynamic_schema;

use diesel::result::Error;
use diesel::sql_types::*;
use diesel::*;
use diesel_dynamic_schema::{schema, table, DynamicSchemaError};

mod dynamic_values;

mod connection_setup;

use connection_setup::{create_posts_table, create_user_table, establish_connection};

#[cfg(feature = "postgres")]
type Backend = diesel::pg::Pg;
#[cfg(feature = "mysql")]
type Backend = diesel::mysql::Mysql;
#[cfg(feature = "mariadb")]
type Backend = diesel::mariadb::Mariadb;
#[cfg(any(feature = "sqlite", feature = "sqlite-no-std"))]
type Backend = diesel::sqlite::Sqlite;

mod static_schema {
    diesel::table! {
        users (id) {
            id -> diesel::sql_types::Integer,
            name -> diesel::sql_types::Text,
        }
    }
    diesel::table! {
        posts (id) {
            id -> diesel::sql_types::Integer,
        }
    }
    diesel::allow_tables_to_appear_in_same_query!(users, posts);
}

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

#[track_caller]
fn assert_foreign_table_error(err: &Error, expected_table: &str) {
    let inner = match err {
        Error::QueryBuilderError(inner) => inner,
        other => panic!("expected QueryBuilderError, got {:?}", other),
    };
    let dynamic = inner
        .downcast_ref::<DynamicSchemaError>()
        .unwrap_or_else(|| panic!("expected DynamicSchemaError, got {}", inner));
    assert!(
        matches!(dynamic, DynamicSchemaError::ForeignTable { .. }),
        "expected ForeignTable, got {:?}",
        dynamic
    );
    assert!(
        dynamic.to_string().contains(expected_table),
        "expected `{}` in `{}`",
        expected_table,
        dynamic
    );
}

#[test]
fn correct_table_select_filter_and_order_execute() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");

    let got = users
        .select(name)
        .filter(name.ne("Tess"))
        .order(id.desc())
        .load::<String>(conn);

    assert_eq!(Ok(vec!["Sean".to_string()]), got);
}

#[test]
fn foreign_table_in_select_is_rejected_before_execution() {
    let conn = &mut establish_connection();
    create_user_table(conn);

    let users = table("users");
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = users.select(posts_id).load::<i32>(conn).unwrap_err();
    assert_foreign_table_error(&err, "posts");
}

#[test]
fn foreign_column_rejected_in_filter_and_order() {
    let conn = &mut establish_connection();
    create_user_table(conn);

    let users = table("users");
    let name = users.column::<Text, _>("name");
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = users
        .select(name)
        .filter(posts_id.eq(1))
        .load::<String>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");

    let err = users
        .select(name)
        .order(posts_id.asc())
        .load::<String>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");
}

#[test]
fn nested_and_correlated_subqueries_resolve_current_and_outer_frames() {
    let conn = &mut establish_connection();
    create_user_table(conn);
    create_posts_table(conn);
    sql_query("INSERT INTO users (name) VALUES ('Sean')")
        .execute(conn)
        .unwrap();

    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");
    let posts = table("posts");
    let post_user = posts.column::<Integer, _>("user_id");

    let user_id = users.select(id).first::<i32>(conn).unwrap();
    sql_query(format!("INSERT INTO posts (user_id) VALUES ({user_id})"))
        .execute(conn)
        .unwrap();

    let nested = users
        .select(name)
        .filter(id.eq_any(posts.select(post_user)))
        .load::<String>(conn);
    assert_eq!(Ok(vec!["Sean".to_string()]), nested);

    let correlated = users
        .select(name)
        .filter(diesel::dsl::exists(
            posts.select(post_user).filter(post_user.eq(id)),
        ))
        .load::<String>(conn);
    assert_eq!(Ok(vec!["Sean".to_string()]), correlated);

    let other = table("other");
    let other_col = other.column::<Integer, _>("x");
    let err = users
        .select(name)
        .filter(diesel::dsl::exists(
            posts.select(post_user).filter(other_col.eq(id)),
        ))
        .load::<String>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "other");
}

#[test]
fn root_query_without_source_rejects_dynamic_column() {
    let conn = &mut establish_connection();
    create_user_table(conn);

    let users = table("users");
    let name = users.column::<Text, _>("name");

    let err = diesel::select(name).load::<String>(conn).unwrap_err();
    assert_foreign_table_error(&err, "users");
}

#[test]
fn standalone_fragment_render_has_no_statement_context() {
    let users = table("users");
    let name = users.column::<Text, _>("name");
    let rendered = debug_query::<Backend, _>(&name).to_string();

    #[cfg(feature = "postgres")]
    assert_eq!(r#""users"."name" -- binds: []"#, rendered);
    #[cfg(not(feature = "postgres"))]
    assert_eq!("`users`.`name` -- binds: []", rendered);
}

#[test]
fn schema_qualified_tables_compare_both_components() {
    let conn = &mut establish_connection();
    create_user_table(conn);

    let from_a = schema("a").table("users");
    let from_b = schema("b").table("users");
    let b_name = from_b.column::<Text, _>("name");

    let err = from_a.select(b_name).load::<String>(conn).unwrap_err();
    assert_foreign_table_error(&err, "b.users");

    let a_name = from_a.column::<Text, _>("name");
    let rendered = debug_query::<Backend, _>(&from_a.select(a_name)).to_string();
    assert!(
        rendered.contains("users"),
        "schema-qualified select should render, got `{}`",
        rendered
    );
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

#[test]
fn foreign_column_rejected_in_group_by_and_having() {
    use diesel::dsl::count;

    let conn = &mut establish_connection();
    create_user_table(conn);

    let users = table("users");
    let name = users.column::<Text, _>("name");
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = users
        .select(name)
        .group_by(posts_id)
        .load::<String>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");

    let err = users
        .select(name)
        .group_by(name)
        .having(count(posts_id).gt(1))
        .load::<String>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");
}

#[test]
fn foreign_column_rejected_in_update_where() {
    let conn = &mut establish_connection();
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = diesel::update(static_schema::users::table)
        .set(static_schema::users::name.eq("x"))
        .filter(posts_id.eq(1))
        .execute(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");
}

#[test]
fn foreign_column_rejected_in_delete_where() {
    let conn = &mut establish_connection();
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = diesel::delete(static_schema::users::table)
        .filter(posts_id.eq(1))
        .execute(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");
}

#[test]
fn foreign_column_rejected_over_static_join() {
    use diesel::query_dsl::JoinOnDsl;

    let conn = &mut establish_connection();
    let other = table("other");
    let other_col = other.column::<Integer, _>("x");

    let err = static_schema::users::table
        .inner_join(
            static_schema::posts::table.on(static_schema::users::id.eq(static_schema::posts::id)),
        )
        .select(other_col)
        .load::<i32>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "other");
}

#[test]
#[cfg(feature = "postgres")]
fn foreign_column_rejected_in_insert_returning() {
    let conn = &mut establish_connection();
    let posts = table("posts");
    let posts_id = posts.column::<Integer, _>("id");

    let err = diesel::insert_into(static_schema::users::table)
        .values(static_schema::users::name.eq("x"))
        .returning(posts_id)
        .get_result::<i32>(conn)
        .unwrap_err();
    assert_foreign_table_error(&err, "posts");
}
