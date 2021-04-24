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
    }

    assert_eq!(
        expected_sql,
        debug_query::<TestBackend, _>(&source).to_string()
    );
    let conn = connection();

    assert!(source.execute(&conn).is_ok());
}

#[test]
fn simple_having() {
    let connection = connection();
    connection
        .execute("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .unwrap();
    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess')",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess')",
        )
        .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .having(diesel::dsl::count(comments::id).eq(2))
        .select((users::name, posts::title));

    let expected_data = vec![("Tess".to_string(), "Hi Tess".to_string())];
    let data: Vec<(String, String)> = source.load(&connection).unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn multi_condition_having() {
    let connection = connection();
    connection
        .execute("INSERT INTO users (id, name, hair_color) VALUES (1, 'Sean', 'red'), (2, 'Tess', 'red'), \
        (3, 'Nick', 'black')")
        .unwrap();
    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES (1, 1, 'Hi Sean'), (2, 2, 'Hi Tess'), (3, 3, 'Hi Nick')",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO comments (id, post_id, text) VALUES (1, 1, 'Comment for Hi Sean'), \
        (2, 2, 'Comment for Hi Tess'), (3, 2, 'Another comment for Hi Tess'), \
        (4, 3, 'Comment for Hi Nick'), (5, 3, 'Another comment for Hi Nick')",
        )
        .unwrap();

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .having(
            diesel::dsl::count(comments::id)
                .eq(2)
                .and(users::hair_color.eq("black")),
        )
        .select((users::name, posts::title));

    let expected_data = vec![("Nick".to_string(), "Hi Nick".to_string())];
    let data: Vec<(String, String)> = source.load(&connection).unwrap();

    assert_eq!(expected_data, data);
}
