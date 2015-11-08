use super::schema::*;
use yaqb::*;

#[test]
fn belongs_to() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (2, 'World', DEFAULT)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", Some("Content"));
    let tess_post = Post::new(2, 2, "World", None);

    let expected_data = vec![(seans_post, sean), (tess_post, tess)];
    let source = posts::table.inner_join(users::table);
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_single_from_join() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (2, 'World')
    ").unwrap();

    let source = posts::table.inner_join(users::table);
    let select_name = source.select(users::name);
    let select_title = source.select(posts::title);

    let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
    let actual_names: Vec<String> = connection.query_all(select_name).unwrap().collect();

    assert_eq!(expected_names, actual_names);

    let expected_titles = vec!["Hello".to_string(), "World".to_string()];
    let actual_titles: Vec<String> = connection.query_all(select_title).unwrap().collect();

    assert_eq!(expected_titles, actual_titles);
}

#[test]
fn select_multiple_from_join() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (2, 'World')
    ").unwrap();

    let source = posts::table.inner_join(users::table)
        .select((users::name, posts::title));

    let expected_data = vec![
        ("Sean".to_string(), "Hello".to_string()),
        ("Tess".to_string(), "World".to_string()),
    ];
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_only_one_side_of_join() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES (2, 'Hello')")
        .unwrap();

    let source = users::table.inner_join(posts::table).select(users::star);

    let expected_data = vec![User::new(2, "Tess")];
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn left_outer_joins() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", None);
    let seans_second_post = Post::new(2, 1, "World", None);

    let expected_data = vec![
        (sean.clone(), Some(seans_post)),
        (sean, Some(seans_second_post)),
        (tess, None)
    ];
    let source = users::table.left_outer_join(posts::table);
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn columns_on_right_side_of_left_outer_joins_are_nullable() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ").unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Hello".to_string())),
        ("Sean".to_string(), Some("World".to_string())),
        ("Tess".to_string(), None),
    ];
    let source = users::table.left_outer_join(posts::table).select((users::name, posts::title));
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_multiple_from_right_side_returns_optional_tuple() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', DEFAULT)
    ").unwrap();

    let expected_data = vec![
        Some(("Hello".to_string(), Some("Content".to_string()))),
        Some(("World".to_string(), None)),
        None,
    ];

    let source = users::table.left_outer_join(posts::table).select((posts::title, posts::body));
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_complex_from_left_join() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', DEFAULT)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (sean.clone(), Some(("Hello".to_string(), Some("Content".to_string())))),
        (sean, Some(("World".to_string(), None))),
        (tess, None),
    ];

    let source = users::table.left_outer_join(posts::table).select((users::star, (posts::title, posts::body)));
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_right_side_with_nullable_column_first() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', DEFAULT)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (sean.clone(), Some((Some("Content".to_string()), "Hello".to_string()))),
        (sean, Some((None, "World".to_string()))),
        (tess, None),
    ];

    let source = users::table.left_outer_join(posts::table).select((users::star, (posts::body, posts::title)));
    let actual_data: Vec<_> = connection.query_all(source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}
