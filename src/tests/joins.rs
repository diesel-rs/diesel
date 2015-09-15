use super::schema::*;
use {Table, QuerySource};

#[test]
fn belongs_to() {
    let connection = connection();
    setup_users_table(&connection);
    setup_posts_table(&connection);

    connection.execute("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .unwrap();
    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (2, 'World')
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post { id: 1, user_id: 1, title: "Hello".to_string() };
    let tess_post = Post { id: 2, user_id: 2, title: "World".to_string() };

    let expected_data = vec![(seans_post, sean), (tess_post, tess)];
    let source = posts::table.inner_join(users::table);
    let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

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
    let actual_names: Vec<String> = connection.query_all(&select_name).unwrap().collect();

    assert_eq!(expected_names, actual_names);

    let expected_titles = vec!["Hello".to_string(), "World".to_string()];
    let actual_titles: Vec<String> = connection.query_all(&select_title).unwrap().collect();

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
    let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

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
    let actual_data: Vec<_> = connection.query_all(&source).unwrap().collect();

    assert_eq!(expected_data, actual_data);
}
