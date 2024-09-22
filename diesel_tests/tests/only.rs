use crate::schema::connection;
use diesel::dsl::*;
use diesel::prelude::*;

// The reason we declare tables specifically for these tests, is that we test inheritance and thus
// need two tables for each table to that end, along with an extra column `archived` (although
// maybe not strictly necessary to test the `ONLY` feature, it serves the use case of using
// inheritance for archival)

table! {
    users (id) {
        id -> Int4,
        name -> Varchar,
        archived -> Bool,
    }
}
table! {
    users_archived (id) {
        id -> Int4,
        name -> Varchar,
        archived -> Bool,
    }
}
table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        body -> Nullable<Text>,
        tags -> Array<Text>,
        archived -> Bool,
    }
}
table! {
    posts_archived (id) {
        id -> Int4,
        user_id -> Int4,
        title -> Varchar,
        body -> Nullable<Text>,
        tags -> Array<Text>,
        archived -> Bool,
    }
}
joinable!(posts -> users (user_id));
allow_tables_to_appear_in_same_query!(users, users_archived, posts, posts_archived);

fn setup_tables(connection: &mut PgConnection) {
    // NOTE: In these tests, we don't use foreign key constraints for maximum flexibility.
    // The reason is that a real FK cannot reference an entry in an inherited table (e.g.
    // posts_archived), while we want to do so in these tests.
    for table in &["users", "users_archived", "posts", "posts_archived"] {
        diesel::sql_query(format!("DROP TABLE IF EXISTS {table} CASCADE"))
            .execute(connection)
            .unwrap();
    }
    sql_query(
        "CREATE TABLE users (
            id serial primary key,
            name varchar not null,
            archived bool not null default false
        )",
    )
    .execute(connection)
    .unwrap();

    sql_query("CREATE TABLE users_archived (check (archived = true)) inherits (users);")
        .execute(connection)
        .unwrap();

    sql_query(
        "CREATE TABLE posts (
            id serial primary key,
            user_id integer not null,
            title varchar not null,
            body text,
            tags text[],
            archived bool not null default false
        )",
    )
    .execute(connection)
    .unwrap();

    sql_query("CREATE TABLE posts_archived (check (archived = true)) inherits (posts);")
        .execute(connection)
        .unwrap();
}

fn test_scenario(connection: &mut PgConnection) {
    // Two users - one is archived.
    // Each user has two posts - one archived each.
    let uid: i32 = diesel::insert_into(users::table)
        .values(users::name.eq("Sean"))
        .returning(users::id)
        .get_result(connection)
        .unwrap();
    assert_eq!(uid, 1);

    let uid: i32 = diesel::insert_into(users_archived::table)
        .values((
            users_archived::name.eq("Tess"),
            users_archived::archived.eq(true),
        ))
        .returning(users_archived::id)
        .get_result(connection)
        .unwrap();
    assert_eq!(uid, 2);

    for user_id in [1, 2] {
        diesel::insert_into(posts::table)
            .values((posts::user_id.eq(user_id), posts::title.eq("Post")))
            .execute(connection)
            .unwrap();
        diesel::insert_into(posts_archived::table)
            .values((
                posts_archived::user_id.eq(user_id),
                posts_archived::title.eq("Archived post"),
                posts_archived::archived.eq(true),
            ))
            .execute(connection)
            .unwrap();
    }
}

#[derive(Debug, PartialEq, Eq, Queryable, Clone, Insertable, AsChangeset, Selectable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub name: String,
}

#[test]
fn select_from_only_with_inherited_table() {
    let connection = &mut connection();
    setup_tables(connection);
    test_scenario(connection);

    // There is now only one entry in the users_archived table, none in the users table.

    let n_users = users::table
        .select(count(users::id))
        .first::<i64>(connection)
        .unwrap();
    assert_eq!(n_users, 2);

    let n_users_in_main_table = users::table
        .only()
        .select(count(users::id))
        .first::<i64>(connection)
        .unwrap();
    assert_eq!(n_users_in_main_table, 1);
}

#[test]
fn select_from_only_filtering_and_find() {
    // Test that it's possible to call `.only().filter(..)`
    let connection = &mut connection();
    setup_tables(connection);
    test_scenario(connection);

    assert_eq!(
        users::table
            .only()
            .filter(users::name.eq("Sean"))
            .select(users::name)
            .load::<String>(connection)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        users::table
            .only()
            .filter(users::name.eq("Tess"))
            .select(users::name)
            .load::<String>(connection)
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        users::table
            .only()
            .find(1)
            .count()
            .get_result::<i64>(connection)
            .unwrap(),
        1
    );
}

#[test]
fn inner_join_only() {
    // Test that it's possible to call:
    // - `.only().inner_join(X::table)`
    // - `.inner_join(X::table.only())`
    // - `.only().inner_join(X::table.only())`

    let connection = &mut connection();
    setup_tables(connection);
    test_scenario(connection);

    // Exclude archived users
    let results: Vec<(String, String)> = users::table
        .only()
        .inner_join(posts::table)
        .select((users::name, posts::title))
        .load(connection)
        .unwrap();
    assert_eq!(
        results,
        vec![
            ("Sean".to_string(), "Post".to_string()),
            ("Sean".to_string(), "Archived post".to_string())
        ]
    );

    // Exclude archived posts
    let results: Vec<(String, String)> = users::table
        .inner_join(posts::table.only())
        .select((users::name, posts::title))
        .load(connection)
        .unwrap();
    assert_eq!(
        results,
        vec![
            ("Sean".to_string(), "Post".to_string()),
            ("Tess".to_string(), "Post".to_string())
        ]
    );

    // Exclude archived users and posts
    let results: Vec<(String, String)> = users::table
        .only()
        .inner_join(posts::table.only())
        .select((users::name, posts::title))
        .load(connection)
        .unwrap();
    assert_eq!(results, vec![("Sean".to_string(), "Post".to_string())]);
}
