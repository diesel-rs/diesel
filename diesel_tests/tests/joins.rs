use super::schema::*;
use diesel::*;

#[test]
fn belongs_to() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (id, user_id, title, body) VALUES
        (1, 1, 'Hello', 'Content'),
        (2, 2, 'World', NULL)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", Some("Content"));
    let tess_post = Post::new(2, 2, "World", None);

    let expected_data = vec![(seans_post, sean), (tess_post, tess)];
    let source = posts::table.inner_join(users::table);
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_single_from_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ").unwrap();

    let source = posts::table.inner_join(users::table);
    let select_name = source.select(users::name);
    let select_title = source.select(posts::title);

    let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
    let actual_names: Vec<String> = select_name.load(&connection).unwrap();

    assert_eq!(expected_names, actual_names);

    let expected_titles = vec!["Hello".to_string(), "World".to_string()];
    let actual_titles: Vec<String> = select_title.load(&connection).unwrap();

    assert_eq!(expected_titles, actual_titles);
}

#[test]
fn select_multiple_from_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ").unwrap();

    let source = posts::table.inner_join(users::table)
        .select((users::name, posts::title));

    let expected_data = vec![
        ("Sean".to_string(), "Hello".to_string()),
        ("Tess".to_string(), "World".to_string()),
    ];
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_only_one_side_of_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title) VALUES (2, 'Hello')")
        .unwrap();

    let source = users::table.inner_join(posts::table).select(users::all_columns);

    let expected_data = vec![User::new(2, "Tess")];
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn left_outer_joins() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 1, 'World')
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
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn columns_on_right_side_of_left_outer_joins_are_nullable() {
    let connection = connection_with_sean_and_tess_in_users_table();

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
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn columns_on_right_side_of_left_outer_joins_can_be_used_in_filter() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ").unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Hello".to_string())),
    ];
    let source = users::table.left_outer_join(posts::table)
        .select((users::name, posts::title))
        .filter(posts::title.eq("Hello"));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_multiple_from_right_side_returns_optional_tuple_when_nullable_is_called() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ").unwrap();

    let expected_data = vec![
        Some(("Hello".to_string(), Some("Content".to_string()))),
        Some(("World".to_string(), None)),
        None,
    ];

    let source = users::table.left_outer_join(posts::table).select((posts::title, posts::body).nullable());
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_complex_from_left_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (sean.clone(), Some(("Hello".to_string(), Some("Content".to_string())))),
        (sean, Some(("World".to_string(), None))),
        (tess, None),
    ];

    let source = users::table.left_outer_join(posts::table).select((users::all_columns, (posts::title, posts::body).nullable()));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_right_side_with_nullable_column_first() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ").unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (sean.clone(), Some((Some("Content".to_string()), "Hello".to_string()))),
        (sean, Some((None, "World".to_string()))),
        (tess, None),
    ];

    let source = users::table.left_outer_join(posts::table).select((users::all_columns, (posts::body, posts::title).nullable()));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_then_join() {
    use schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    connection.execute("INSERT INTO posts (user_id, title) VALUES (1, 'Hello')")
        .unwrap();
    let expected_data = vec![1];
    let data: Vec<i32> = users.select(id).inner_join(posts::table)
        .load(&connection).unwrap();

    assert_eq!(expected_data, data);

    let expected_data = vec![1, 2];
    let data: Vec<i32> = users.select(id).left_outer_join(posts::table)
        .load(&connection).unwrap();

    assert_eq!(expected_data, data);
}

// FIXME: This is fixed by the new join design which removes the associated type
// form `SelectableExpression`
// #[test]
// fn selecting_complex_expression_from_right_side_of_join() {
//     use diesel::types::Text;

//     let connection = connection_with_sean_and_tess_in_users_table();
//     let new_posts = vec![
//         NewPost::new(1, "Post One", None),
//         NewPost::new(1, "Post Two", None),
//     ];
//     insert(&new_posts).into(posts::table).execute(&connection).unwrap();
//     sql_function!(lower, lower_t, (x: Text) -> Text);

//     let titles = users::table.left_outer_join(posts::table)
//         .select(lower(posts::title).nullable())
//         .order((users::id, posts::id))
//         .load(&connection);
//     let expected_data = vec![Some("post one".to_string()), Some("post two".to_string()), None];
//     assert_eq!(Ok(expected_data), titles);
// }

#[test]
fn join_through_other() {
    use schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    insert(&NewUser::new("Jim", None)).into(users).execute(&connection).unwrap();
    insert(&vec![
        NewPost::new(1, "Hello", None), NewPost::new(2, "World", None),
        NewPost::new(1, "Hello again!", None),
    ]).into(posts::table).execute(&connection).unwrap();
    let posts = posts::table.load::<Post>(&connection).unwrap();
    insert(&vec![
        NewComment(posts[0].id, "OMG"), NewComment(posts[1].id, "WTF"),
        NewComment(posts[2].id, "Best post ever!!!")
    ]).into(comments::table).execute(&connection).unwrap();
    let comments = comments::table.load::<Comment>(&connection).unwrap();

    let data = users.inner_join(comments::table).load(&connection)
        .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (sean.clone(), comments[0].clone()),
        (tess, comments[1].clone()),
        (sean, comments[2].clone()),
    ];
    assert_eq!(expected_data, data);
}
