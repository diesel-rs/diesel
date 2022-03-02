use crate::schema::*;
use diesel::*;

#[test]
fn having_generates_having_sql() {
    let source = users::table
        .inner_join(posts::table)
        .group_by(users::id)
        .having(diesel::dsl::count(posts::id).gt(1))
        .select((users::name, diesel::dsl::count(posts::id)));

    let mut expected_sql = "SELECT `users`.`name`, count(`posts`.`id`) \
    FROM (`users` INNER JOIN `posts` ON (`posts`.`user_id` = `users`.`id`)) \
    GROUP BY `users`.`id` \
    HAVING (count(`posts`.`id`) > ?) -- binds: [1]"
        .to_string();

    if cfg!(feature = "postgres") {
        expected_sql = expected_sql.replace('`', "\"");
        expected_sql = expected_sql.replace('?', "$1");
    }

    assert_eq!(
        expected_sql,
        debug_query::<TestBackend, _>(&source).to_string()
    );
    let conn = &mut connection();

    assert!(source.execute(conn).is_ok());
}

#[test]
fn simple_having_with_group_by() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .execute(connection)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess')",
    )
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess')",
    )
    .execute(connection)
    .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .having(diesel::dsl::count(comments::id).eq(2))
        .select((users::name, posts::title));

    let expected_data = vec![("Tess".to_string(), "Hi Tess".to_string())];
    let data: Vec<(String, String)> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn boxed_simple_having_with_group_by() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .execute(connection)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess')",
    )
    .execute(connection)
    .unwrap();
    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess')",
    )
    .execute(connection)
    .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .having(diesel::dsl::count(comments::id).eq(2))
        .select((users::name, posts::title))
        .into_boxed();

    let expected_data = vec![("Tess".to_string(), "Hi Tess".to_string())];
    let data: Vec<(String, String)> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn multi_condition_having_with_group_by() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess'), (3, 'Nick')")
        .execute(connection)
        .unwrap();
    diesel::sql_query(
            "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess'), (3, 3, 'Hi Nick')",
        ).execute(connection)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess'), \
        (4, 3, 'Comment for Hi Nick'), (5, 3, 'Another comment for Hi Nick')",
    )
    .execute(connection)
    .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .having(diesel::dsl::count(comments::id).eq(2).and(users::id.eq(3)))
        .select((users::id, users::name, posts::title));

    let expected_data = vec![(3, "Nick".to_string(), "Hi Nick".to_string())];
    let data: Vec<(i32, String, String)> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn boxed_multi_condition_having_with_group_by() {
    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess'), (3, 'Nick')")
        .execute(connection)
        .unwrap();
    diesel::sql_query(
            "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess'), (3, 3, 'Hi Nick')",
        ).execute(connection)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess'), \
        (4, 3, 'Comment for Hi Nick'), (5, 3, 'Another comment for Hi Nick')",
    )
    .execute(connection)
    .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .select((users::id, users::name, posts::title))
        .group_by((users::id, posts::id))
        .into_boxed()
        .having(diesel::dsl::count(comments::id).eq(2).and(users::id.eq(3)));

    let expected_data = vec![(3, "Nick".to_string(), "Hi Nick".to_string())];
    let data: Vec<(i32, String, String)> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}
