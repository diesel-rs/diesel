use super::schema::*;
use diesel::*;

#[test]
fn belongs_to() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title, body) VALUES
        (1, 1, 'Hello', 'Content'),
        (2, 2, 'World', NULL)
    ",
        )
        .unwrap();

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

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
        )
        .unwrap();

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

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
        )
        .unwrap();

    let source = posts::table
        .inner_join(users::table)
        .select((users::name, posts::title));

    let expected_data = vec![
        ("Sean".to_string(), "Hello".to_string()),
        ("Tess".to_string(), "World".to_string()),
    ];
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn join_boxed_query() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
        )
        .unwrap();

    let source = posts::table
        .into_boxed()
        .inner_join(users::table)
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

    connection
        .execute("INSERT INTO posts (user_id, title) VALUES (2, 'Hello')")
        .unwrap();

    let source = users::table
        .inner_join(posts::table)
        .select(users::all_columns);

    let expected_data = vec![User::new(2, "Tess")];
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn left_outer_joins() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 1, 'World')
    ",
        )
        .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", None);
    let seans_second_post = Post::new(2, 1, "World", None);

    let expected_data = vec![
        (sean.clone(), Some(seans_post)),
        (sean, Some(seans_second_post)),
        (tess, None),
    ];
    let source = users::table.left_outer_join(posts::table);
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn columns_on_right_side_of_left_outer_joins_are_nullable() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ",
        )
        .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Hello".to_string())),
        ("Sean".to_string(), Some("World".to_string())),
        ("Tess".to_string(), None),
    ];
    let source = users::table
        .left_outer_join(posts::table)
        .select((users::name, posts::title.nullable()));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn columns_on_right_side_of_left_outer_joins_can_be_used_in_filter() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ",
        )
        .unwrap();

    let expected_data = vec![("Sean".to_string(), Some("Hello".to_string()))];
    let source = users::table
        .left_outer_join(posts::table)
        .select((users::name, posts::title.nullable()))
        .filter(posts::title.eq("Hello"));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_multiple_from_right_side_returns_optional_tuple_when_nullable_is_called() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
        )
        .unwrap();

    let expected_data = vec![
        Some(("Hello".to_string(), Some("Content".to_string()))),
        Some(("World".to_string(), None)),
        None,
    ];

    let source = users::table
        .left_outer_join(posts::table)
        .select((posts::title, posts::body).nullable());
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_complex_from_left_join() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
        )
        .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (
            sean.clone(),
            Some(("Hello".to_string(), Some("Content".to_string()))),
        ),
        (sean, Some(("World".to_string(), None))),
        (tess, None),
    ];

    let source = users::table
        .left_outer_join(posts::table)
        .select((users::all_columns, (posts::title, posts::body).nullable()));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_right_side_with_nullable_column_first() {
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute(
            "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
        )
        .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let expected_data = vec![
        (
            sean.clone(),
            Some((Some("Content".to_string()), "Hello".to_string())),
        ),
        (sean, Some((None, "World".to_string()))),
        (tess, None),
    ];

    let source = users::table
        .left_outer_join(posts::table)
        .select((users::all_columns, (posts::body, posts::title).nullable()));
    let actual_data: Vec<_> = source.load(&connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[test]
fn select_then_join() {
    use schema::users::dsl::*;
    let connection = connection_with_sean_and_tess_in_users_table();

    connection
        .execute("INSERT INTO posts (user_id, title) VALUES (1, 'Hello')")
        .unwrap();
    let expected_data = vec![1];
    let data: Vec<i32> = users
        .select(id)
        .inner_join(posts::table)
        .load(&connection)
        .unwrap();

    assert_eq!(expected_data, data);

    let expected_data = vec![1, 2];
    let data: Vec<i32> = users
        .select(id)
        .left_outer_join(posts::table)
        .load(&connection)
        .unwrap();

    assert_eq!(expected_data, data);
}

#[test]
fn selecting_complex_expression_from_right_side_of_left_join() {
    use diesel::sql_types::Text;

    let connection = connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&connection)
        .unwrap();
    sql_function!(fn lower(x: Text) -> Text);

    let titles = users::table
        .left_outer_join(posts::table)
        .select(lower(posts::title).nullable())
        .order((users::id, posts::id))
        .load(&connection);
    let expected_data = vec![
        Some("post one".to_string()),
        Some("post two".to_string()),
        None,
    ];
    assert_eq!(Ok(expected_data), titles);
}

#[test]
fn selecting_complex_expression_from_both_sides_of_outer_join() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&connection)
        .unwrap();

    let titles = users::table
        .left_outer_join(posts::table)
        .select(
            users::name
                .concat(" wrote ")
                .concat(posts::title)
                .nullable(),
        )
        .order((users::id, posts::id))
        .load(&connection);
    let expected_data = vec![
        Some("Sean wrote Post One".to_string()),
        Some("Sean wrote Post Two".to_string()),
        None,
    ];
    assert_eq!(Ok(expected_data), titles);
}

#[test]
fn join_with_explicit_on_clause() {
    let connection = connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&connection)
        .unwrap();

    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);
    let post_one = posts::table
        .filter(posts::title.eq("Post One"))
        .first::<Post>(&connection)
        .unwrap();
    let expected_data = Ok(vec![(sean, post_one.clone()), (tess, post_one)]);

    let data = users::table
        .inner_join(posts::table.on(posts::title.eq("Post One")))
        .load(&connection);

    assert_eq!(expected_data, data);

    let data = users::table
        .inner_join(posts::table.on(posts::title.eq_any(vec!["Post One"])))
        .load(&connection);

    assert_eq!(expected_data, data);
}

#[test]
fn selecting_parent_child_grandchild() {
    let (connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        tess,
        posts,
        comments,
        ..
    } = test_data;

    let data = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (sean.clone(), (posts[0].clone(), comments[0].clone())),
        (sean.clone(), (posts[0].clone(), comments[2].clone())),
        (sean.clone(), (posts[2].clone(), comments[1].clone())),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (sean.clone(), (posts[0].clone(), Some(comments[0].clone()))),
        (sean.clone(), (posts[0].clone(), Some(comments[2].clone()))),
        (sean.clone(), (posts[2].clone(), Some(comments[1].clone()))),
        (tess.clone(), (posts[1].clone(), None)),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .left_outer_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (
            sean.clone(),
            Some((posts[0].clone(), Some(comments[0].clone()))),
        ),
        (
            sean.clone(),
            Some((posts[0].clone(), Some(comments[2].clone()))),
        ),
        (
            sean.clone(),
            Some((posts[2].clone(), Some(comments[1].clone()))),
        ),
        (tess.clone(), Some((posts[1].clone(), None))),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .left_outer_join(posts::table.inner_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (sean.clone(), Some((posts[0].clone(), comments[0].clone()))),
        (sean.clone(), Some((posts[0].clone(), comments[2].clone()))),
        (sean.clone(), Some((posts[2].clone(), comments[1].clone()))),
        (tess.clone(), None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_grandchild_child_parent() {
    let (connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        posts,
        comments,
        ..
    } = test_data;

    let data = comments::table
        .inner_join(posts::table.inner_join(users::table))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (comments[0].clone(), (posts[0].clone(), sean.clone())),
        (comments[2].clone(), (posts[0].clone(), sean.clone())),
        (comments[1].clone(), (posts[2].clone(), sean.clone())),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_four_tables_deep() {
    let (connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        posts,
        comments,
        likes,
        ..
    } = test_data;

    let data = users::table
        .inner_join(posts::table.inner_join(comments::table.inner_join(likes::table)))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![(
        sean.clone(),
        (posts[0].clone(), (comments[0].clone(), likes[0].clone())),
    )];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.inner_join(comments::table.left_outer_join(likes::table)))
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (
            sean.clone(),
            (
                posts[0].clone(),
                (comments[0].clone(), Some(likes[0].clone())),
            ),
        ),
        (
            sean.clone(),
            (posts[0].clone(), (comments[2].clone(), None)),
        ),
        (
            sean.clone(),
            (posts[2].clone(), (comments[1].clone(), None)),
        ),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_parent_child_sibling() {
    let (connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        tess,
        posts,
        likes,
        ..
    } = test_data;

    let data = users::table
        .inner_join(posts::table)
        .inner_join(likes::table)
        .load(&connection);
    let expected = vec![(tess.clone(), posts[1].clone(), likes[0].clone())];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table)
        .left_outer_join(likes::table)
        .order((users::id, posts::id))
        .load(&connection);
    let expected = vec![
        (sean.clone(), posts[0].clone(), None),
        (sean.clone(), posts[2].clone(), None),
        (tess.clone(), posts[1].clone(), Some(likes[0].clone())),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_crazy_nested_joins() {
    let (connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        tess,
        posts,
        likes,
        comments,
        followings,
        ..
    } = test_data;

    let data = users::table
        .inner_join(
            posts::table
                .left_join(comments::table.left_join(likes::table))
                .left_join(followings::table),
        )
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (
            sean.clone(),
            (
                posts[0].clone(),
                Some((comments[0].clone(), Some(likes[0].clone()))),
                None,
            ),
        ),
        (
            sean.clone(),
            (posts[0].clone(), Some((comments[2].clone(), None)), None),
        ),
        (
            sean.clone(),
            (posts[2].clone(), Some((comments[1].clone(), None)), None),
        ),
        (
            tess.clone(),
            (posts[1].clone(), None, Some(followings[0].clone())),
        ),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.left_join(comments::table.left_join(likes::table)))
        .left_join(followings::table)
        .order((users::id, posts::id, comments::id))
        .load(&connection);
    let expected = vec![
        (
            sean.clone(),
            (
                posts[0].clone(),
                Some((comments[0].clone(), Some(likes[0].clone()))),
            ),
            Some(followings[0]),
        ),
        (
            sean.clone(),
            (posts[0].clone(), Some((comments[2].clone(), None))),
            Some(followings[0]),
        ),
        (
            sean.clone(),
            (posts[2].clone(), Some((comments[1].clone(), None))),
            Some(followings[0]),
        ),
        (tess.clone(), (posts[1].clone(), None), None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
#[cfg(diesel_experimental)]
fn joining_to_aliased_table() {
    use diesel::sql_types::Bool;

    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);

    diesel_define_alias!(users2);
    allow_types_to_appear_in_same_query!(users::table, users2);

    let u2 = users::table.aliased(users2);
    let (u2_id, ..) = u2.selection();
    let all_users_twice = users::table.inner_join(u2.on(true.into_sql::<Bool>()))
        .order((users::id, u2_id))
        .load::<(User, User)>(&connection);

    let expected = vec![
        (sean.clone(), sean.clone()),
        (sean.clone(), tess.clone()),
        (tess.clone(), sean.clone()),
        (tess.clone(), tess.clone()),
    ];
    assert_eq!(Ok(expected), all_users_twice);
}

fn connection_with_fixture_data_for_multitable_joins() -> (TestConnection, TestData) {
    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);

    let new_posts = vec![
        NewPost::new(sean.id, "First Post", None),
        NewPost::new(tess.id, "Second Post", None),
        NewPost::new(sean.id, "Third Post", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&connection)
        .unwrap();

    let posts = posts::table
        .order(posts::id)
        .load::<Post>(&connection)
        .unwrap();
    let new_comments = vec![
        NewComment(posts[0].id, "First Comment"),
        NewComment(posts[2].id, "Second Comment"),
        NewComment(posts[0].id, "Third Comment"),
    ];
    insert_into(comments::table)
        .values(&new_comments)
        .execute(&connection)
        .unwrap();

    let comments = comments::table
        .order(comments::id)
        .load::<Comment>(&connection)
        .unwrap();
    let like = Like {
        user_id: tess.id,
        comment_id: *comments[0].id(),
    };
    insert_into(likes::table)
        .values(&like)
        .execute(&connection)
        .unwrap();

    let likes = likes::table
        .order((likes::user_id, likes::comment_id))
        .load(&connection)
        .unwrap();

    let new_following = Following {
        user_id: sean.id,
        post_id: posts[1].id,
        email_notifications: false,
    };
    insert_into(followings::table)
        .values(&new_following)
        .execute(&connection)
        .unwrap();
    let followings = followings::table
        .order((followings::user_id, followings::post_id))
        .load(&connection)
        .unwrap();

    let test_data = TestData {
        sean,
        tess,
        posts,
        comments,
        likes,
        followings,
    };

    (connection, test_data)
}

struct TestData {
    sean: User,
    tess: User,
    posts: Vec<Post>,
    comments: Vec<Comment>,
    likes: Vec<Like>,
    followings: Vec<Following>,
}
