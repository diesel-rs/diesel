use crate::schema::*;
use diesel::sql_types::Text;
use diesel::*;

#[diesel_test_helper::test]
fn belongs_to() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title, body) VALUES
        (1, 1, 'Hello', 'Content'),
        (2, 2, 'World', NULL)
    ",
    )
    .execute(connection)
    .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", Some("Content"));
    let tess_post = Post::new(2, 2, "World", None);

    let expected_data = vec![(seans_post, sean), (tess_post, tess)];
    let source = posts::table.inner_join(users::table).order(posts::id);
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_single_from_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    let source = posts::table.inner_join(users::table);
    let select_name = source.select(users::name).order(users::name);
    let select_title = source.select(posts::title).order(posts::title);

    let expected_names = vec!["Sean".to_string(), "Tess".to_string()];
    let actual_names: Vec<String> = select_name.load(connection).unwrap();

    assert_eq!(expected_names, actual_names);

    let expected_titles = vec!["Hello".to_string(), "World".to_string()];
    let actual_titles: Vec<String> = select_title.load(connection).unwrap();

    assert_eq!(expected_titles, actual_titles);
}

#[diesel_test_helper::test]
fn select_multiple_from_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    let source = posts::table
        .inner_join(users::table)
        .select((users::name, posts::title));

    let expected_data = vec![
        ("Sean".to_string(), "Hello".to_string()),
        ("Tess".to_string(), "World".to_string()),
    ];
    let actual_data: Vec<_> = source.order(users::name).load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn join_boxed_query() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 2, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    let source = posts::table
        .into_boxed()
        .inner_join(users::table)
        .select((users::name, posts::title));

    let expected_data = vec![
        ("Sean".to_string(), "Hello".to_string()),
        ("Tess".to_string(), "World".to_string()),
    ];
    let actual_data: Vec<_> = source.order(users::name).load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_only_one_side_of_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query("INSERT INTO posts (user_id, title) VALUES (2, 'Hello')")
        .execute(connection)
        .unwrap();

    let source = users::table
        .inner_join(posts::table)
        .select(users::all_columns);

    let expected_data = vec![User::new(2, "Tess")];
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn left_outer_joins() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 1, 'World')
    ",
    )
    .execute(connection)
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
    let source = users::table
        .left_outer_join(posts::table)
        .order_by((users::id.asc(), posts::id.asc()));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn full_outer_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO users (id, name) VALUES
        (3, 'James')
    ",
    )
    .execute(connection)
    .unwrap();

    // exclude James in the query to get a null entry without breaking FKs
    diesel::sql_query(
        "CREATE OR REPLACE TEMPORARY VIEW filtered_users AS
        SELECT * FROM users
        WHERE id <> 3
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 1, 'World'),
        (3, 3, 'Again')
    ",
    )
    .execute(connection)
    .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", None);
    let seans_second_post = Post::new(2, 1, "World", None);
    let orphaned_post = Post::new(3, 3, "Again", None);

    let expected_data = vec![
        (Some(sean.clone()), Some(seans_post)),
        (Some(sean), Some(seans_second_post)),
        (Some(tess), None),
        (None, Some(orphaned_post)),
    ];

    let source = filtered_users::table
        .full_outer_join(posts::table.on(posts::user_id.eq(filtered_users::id)))
        .order_by((filtered_users::id.asc(), posts::id.asc()));

    let actual_data = source
        .load::<(Option<User>, Option<Post>)>(connection)
        .unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn multiple_full_outer_joins() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO users (id, name) VALUES
        (3, 'James')
    ",
    )
    .execute(connection)
    .unwrap();

    // exclude James in the query to get a null entry without breaking FKs
    diesel::sql_query(
        "CREATE OR REPLACE TEMPORARY VIEW filtered_users AS
        SELECT * FROM users
        WHERE id <> 3
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 3, 'World'),
        (3, 3, 'Again')
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES
        (1, 1, 'Comment 1'),
        (2, 3, 'Comment 2'),
        (3, 3, 'Comment 3')
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO likes (comment_id, user_id) VALUES
        (2, 2),
        (2, 3)
    ",
    )
    .execute(connection)
    .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", None);
    let orphaned_post_3 = Post::new(3, 3, "Again", None);

    let comment_1 = Comment::new(1, 1, "Comment 1");
    let comment_2 = Comment::new(2, 3, "Comment 2");
    let comment_3 = Comment::new(3, 3, "Comment 3");

    let like_1 = Like {
        comment_id: 2,
        user_id: 2,
    };
    let like_2 = Like {
        comment_id: 2,
        user_id: 3,
    };

    let expected_data = vec![
        (Some(sean), Some(seans_post), Some(comment_1), None),
        (Some(tess), None, None, None),
        (
            None,
            Some(orphaned_post_3.clone()),
            Some(comment_2.clone()),
            Some(like_1),
        ),
        (
            None,
            Some(orphaned_post_3.clone()),
            Some(comment_2),
            Some(like_2),
        ),
        (None, Some(orphaned_post_3), Some(comment_3), None),
    ];
    let source = filtered_users::table
        .full_outer_join(posts::table.on(posts::user_id.eq(filtered_users::id)))
        .full_outer_join(comments::table.on(posts::id.eq(comments::post_id)))
        .full_outer_join(likes::table.on(comments::id.eq(likes::comment_id)))
        .order_by((
            filtered_users::id.asc(),
            posts::id.asc(),
            comments::id.asc(),
            likes::user_id.asc(),
        ));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn full_inner_and_left_joins() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO users (id, name) VALUES
        (3, 'James')
    ",
    )
    .execute(connection)
    .unwrap();

    // exclude James in the query to get a null entry without breaking FKs
    diesel::sql_query(
        "CREATE OR REPLACE TEMPORARY VIEW filtered_users AS
        SELECT * FROM users
        WHERE id <> 3
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO posts (id, user_id, title) VALUES
        (1, 1, 'Hello'),
        (2, 3, 'World'),
        (3, 3, 'Again')
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO comments (id, post_id, text) VALUES
        (1, 1, 'Comment 1'),
        (2, 3, 'Comment 2'),
        (3, 3, 'Comment 3')
    ",
    )
    .execute(connection)
    .unwrap();

    diesel::sql_query(
        "INSERT INTO likes (comment_id, user_id) VALUES
        (2, 2),
        (2, 3)
    ",
    )
    .execute(connection)
    .unwrap();

    let sean = User::new(1, "Sean");
    let seans_post = Post::new(1, 1, "Hello", None);
    let orphaned_post_3 = Post::new(3, 3, "Again", None);

    let comment_1 = Comment::new(1, 1, "Comment 1");
    let comment_2 = Comment::new(2, 3, "Comment 2");
    let comment_3 = Comment::new(3, 3, "Comment 3");

    let like_1 = Like {
        comment_id: 2,
        user_id: 2,
    };
    let like_2 = Like {
        comment_id: 2,
        user_id: 3,
    };

    // all posts and users, but only with a post with a comment. but any likes
    // this means only post 1 & 3, but only 3 has likes

    let expected_data = vec![
        (Some(sean), Some(seans_post), comment_1, None),
        (
            None,
            Some(orphaned_post_3.clone()),
            comment_2.clone(),
            Some(like_1),
        ),
        (None, Some(orphaned_post_3.clone()), comment_2, Some(like_2)),
        (None, Some(orphaned_post_3), comment_3, None),
    ];
    let source = filtered_users::table
        .full_outer_join(posts::table.on(posts::user_id.eq(filtered_users::id)))
        .inner_join(comments::table.on(posts::id.eq(comments::post_id)))
        .left_join(likes::table.on(comments::id.eq(likes::comment_id)))
        .order_by((
            filtered_users::id.asc(),
            posts::id.asc(),
            comments::id.asc(),
            likes::user_id.asc(),
        ));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn columns_on_right_side_of_left_outer_joins_are_nullable() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    let expected_data = vec![
        ("Sean".to_string(), Some("Hello".to_string())),
        ("Sean".to_string(), Some("World".to_string())),
        ("Tess".to_string(), None),
    ];
    let source = users::table
        .left_outer_join(posts::table)
        .select((users::name, posts::title.nullable()))
        .order_by((users::id.asc(), posts::title.asc()));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn columns_on_right_side_of_left_outer_joins_can_be_used_in_filter() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title) VALUES
        (1, 'Hello'),
        (1, 'World')
    ",
    )
    .execute(connection)
    .unwrap();

    let expected_data = vec![("Sean".to_string(), Some("Hello".to_string()))];
    let source = users::table
        .left_outer_join(posts::table)
        .select((users::name, posts::title.nullable()))
        .filter(posts::title.eq("Hello"));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_multiple_from_right_side_returns_optional_tuple_when_nullable_is_called() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
    )
    .execute(connection)
    .unwrap();

    let expected_data = vec![
        Some(("Hello".to_string(), Some("Content".to_string()))),
        Some(("World".to_string(), None)),
        None,
    ];

    let source = users::table
        .left_outer_join(posts::table)
        .select((posts::title, posts::body).nullable())
        .order_by((users::id.asc(), posts::id.asc()));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_complex_from_left_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
    )
    .execute(connection)
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
        .select((users::all_columns, (posts::title, posts::body).nullable()))
        .order_by((users::id.asc(), posts::id.asc()));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_right_side_with_nullable_column_first() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content'),
        (1, 'World', NULL)
    ",
    )
    .execute(connection)
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
        .select((users::all_columns, (posts::body, posts::title).nullable()))
        .order_by((users::id.asc(), posts::id.asc()));
    let actual_data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
#[allow(clippy::type_complexity)]
fn select_left_join_right_side_with_non_null_inside() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query(
        "INSERT INTO posts (user_id, title, body) VALUES
        (1, 'Hello', 'Content')
    ",
    )
    .execute(connection)
    .unwrap();

    let expected_data = vec![
        (None, 2),
        (Some((1, "Hello".to_string(), "Hello".to_string())), 1),
    ];

    let source = users::table
        .left_outer_join(posts::table)
        .select((
            (users::id, posts::title, posts::title).nullable(),
            users::id,
        ))
        .order_by((users::id.desc(), posts::id.asc()));
    let actual_data: Vec<(Option<(i32, String, String)>, i32)> = source.load(connection).unwrap();

    assert_eq!(expected_data, actual_data);
}

#[diesel_test_helper::test]
fn select_then_join() {
    use crate::schema::users::dsl::*;
    let connection = &mut connection_with_sean_and_tess_in_users_table();

    diesel::sql_query("INSERT INTO posts (user_id, title) VALUES (1, 'Hello')")
        .execute(connection)
        .unwrap();
    let expected_data = vec![1];
    let data: Vec<i32> = users
        .select(id)
        .inner_join(posts::table)
        .load(connection)
        .unwrap();

    assert_eq!(expected_data, data);

    let expected_data = vec![1, 2];
    let data: Vec<i32> = users
        .select(id)
        .order(id)
        .left_outer_join(posts::table)
        .load(connection)
        .unwrap();

    assert_eq!(expected_data, data);
}

#[declare_sql_function]
extern "SQL" {
    fn lower(x: Text) -> Text;
}

#[diesel_test_helper::test]
fn selecting_complex_expression_from_right_side_of_left_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(connection)
        .unwrap();

    let titles = users::table
        .left_outer_join(posts::table)
        .select(lower(posts::title).nullable())
        .order((users::id, posts::id))
        .load(connection);
    let expected_data = vec![
        Some("post one".to_string()),
        Some("post two".to_string()),
        None,
    ];
    assert_eq!(Ok(expected_data), titles);
}

#[diesel_test_helper::test]
fn selecting_complex_expression_from_both_sides_of_outer_join() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(connection)
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
        .load(connection);
    let expected_data = vec![
        Some("Sean wrote Post One".to_string()),
        Some("Sean wrote Post Two".to_string()),
        None,
    ];
    assert_eq!(Ok(expected_data), titles);
}

#[diesel_test_helper::test]
fn join_with_explicit_on_clause() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let new_posts = vec![
        NewPost::new(1, "Post One", None),
        NewPost::new(1, "Post Two", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(connection)
        .unwrap();

    let sean = find_user_by_name("Sean", connection);
    let tess = find_user_by_name("Tess", connection);
    let post_one = posts::table
        .filter(posts::title.eq("Post One"))
        .first::<Post>(connection)
        .unwrap();
    let expected_data = Ok(vec![(sean, post_one.clone()), (tess, post_one)]);

    let data = users::table
        .inner_join(posts::table.on(posts::title.eq("Post One")))
        .order(users::id)
        .load(connection);

    assert_eq!(expected_data, data);

    let data = users::table
        .inner_join(posts::table.on(posts::title.eq_any(vec!["Post One"])))
        .order(users::id)
        .load(connection);

    assert_eq!(expected_data, data);
}

#[diesel_test_helper::test]
fn selecting_parent_child_grandchild() {
    let (mut connection, test_data) = connection_with_fixture_data_for_multitable_joins();
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
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), (posts[0].clone(), comments[0].clone())),
        (sean.clone(), (posts[0].clone(), comments[2].clone())),
        (sean.clone(), (posts[2].clone(), comments[1].clone())),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(
            posts::table
                .on(users::id.eq(posts::user_id).and(posts::id.eq(posts[0].id)))
                .inner_join(comments::table),
        )
        .order((users::id, posts::id, comments::id))
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), (posts[0].clone(), comments[0].clone())),
        (sean.clone(), (posts[0].clone(), comments[2].clone())),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .load(&mut connection);
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
        .load(&mut connection);
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
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), Some((posts[0].clone(), comments[0].clone()))),
        (sean.clone(), Some((posts[0].clone(), comments[2].clone()))),
        (sean, Some((posts[2].clone(), comments[1].clone()))),
        (tess, None),
    ];
    assert_eq!(Ok(expected), data);
}

#[diesel_test_helper::test]
fn selecting_grandchild_child_parent() {
    let (mut connection, test_data) = connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        posts,
        comments,
        ..
    } = test_data;

    let data = comments::table
        .inner_join(posts::table.inner_join(users::table))
        .order((users::id, posts::id, comments::id))
        .load(&mut connection);
    let expected = vec![
        (comments[0].clone(), (posts[0].clone(), sean.clone())),
        (comments[2].clone(), (posts[0].clone(), sean.clone())),
        (comments[1].clone(), (posts[2].clone(), sean)),
    ];
    assert_eq!(Ok(expected), data);
}

#[diesel_test_helper::test]
fn selecting_four_tables_deep() {
    let (mut connection, test_data) = connection_with_fixture_data_for_multitable_joins();
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
        .load(&mut connection);
    let expected = vec![(
        sean.clone(),
        (posts[0].clone(), (comments[0].clone(), likes[0])),
    )];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.inner_join(comments::table.left_outer_join(likes::table)))
        .order((users::id, posts::id, comments::id))
        .load(&mut connection);
    let expected = vec![
        (
            sean.clone(),
            (posts[0].clone(), (comments[0].clone(), Some(likes[0]))),
        ),
        (
            sean.clone(),
            (posts[0].clone(), (comments[2].clone(), None)),
        ),
        (sean, (posts[2].clone(), (comments[1].clone(), None))),
    ];
    assert_eq!(Ok(expected), data);
}

#[diesel_test_helper::test]
fn selecting_parent_child_sibling() {
    let (mut connection, test_data) = connection_with_fixture_data_for_multitable_joins();
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
        .load(&mut connection);
    let expected = vec![(tess.clone(), posts[1].clone(), likes[0])];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table)
        .left_outer_join(likes::table)
        .order((users::id, posts::id))
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), posts[0].clone(), None),
        (sean, posts[2].clone(), None),
        (tess, posts[1].clone(), Some(likes[0])),
    ];
    assert_eq!(Ok(expected), data);
}

#[diesel_test_helper::test]
fn selecting_crazy_nested_joins() {
    let (mut connection, test_data) = connection_with_fixture_data_for_multitable_joins();
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
        .load(&mut connection);
    let expected = vec![
        (
            sean.clone(),
            (
                posts[0].clone(),
                Some((comments[0].clone(), Some(likes[0]))),
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
        (tess.clone(), (posts[1].clone(), None, Some(followings[0]))),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.left_join(comments::table.left_join(likes::table)))
        .left_join(followings::table)
        .order((users::id, posts::id, comments::id))
        .load(&mut connection);
    let expected = vec![
        (
            sean.clone(),
            (
                posts[0].clone(),
                Some((comments[0].clone(), Some(likes[0]))),
            ),
            Some(followings[0]),
        ),
        (
            sean.clone(),
            (posts[0].clone(), Some((comments[2].clone(), None))),
            Some(followings[0]),
        ),
        (
            sean,
            (posts[2].clone(), Some((comments[1].clone(), None))),
            Some(followings[0]),
        ),
        (tess, (posts[1].clone(), None), None),
    ];
    assert_eq!(Ok(expected), data);
}

pub(crate) fn connection_with_fixture_data_for_multitable_joins() -> (TestConnection, TestData) {
    let mut connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &mut connection);
    let tess = find_user_by_name("Tess", &mut connection);

    let new_posts = vec![
        NewPost::new(sean.id, "First Post", None),
        NewPost::new(tess.id, "Second Post", None),
        NewPost::new(sean.id, "Third Post", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&mut connection)
        .unwrap();

    let posts = posts::table
        .order(posts::id)
        .load::<Post>(&mut connection)
        .unwrap();
    let new_comments: &[NewComment<'static>] = &[
        NewComment(posts[0].id, "First Comment"),
        NewComment(posts[2].id, "Second Comment"),
        NewComment(posts[0].id, "Third Comment"),
    ];
    insert_into(comments::table)
        .values(new_comments)
        .execute(&mut connection)
        .unwrap();

    let comments = comments::table
        .order(comments::id)
        .load::<Comment>(&mut connection)
        .unwrap();
    let like = Like {
        user_id: tess.id,
        comment_id: *comments[0].id(),
    };
    insert_into(likes::table)
        .values(&like)
        .execute(&mut connection)
        .unwrap();

    let likes = likes::table
        .order((likes::user_id, likes::comment_id))
        .load(&mut connection)
        .unwrap();

    let new_following = Following {
        user_id: sean.id,
        post_id: posts[1].id,
        email_notifications: false,
    };
    insert_into(followings::table)
        .values(&new_following)
        .execute(&mut connection)
        .unwrap();
    let followings = followings::table
        .order((followings::user_id, followings::post_id))
        .load(&mut connection)
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

pub struct TestData {
    pub sean: User,
    pub tess: User,
    pub posts: Vec<Post>,
    pub comments: Vec<Comment>,
    pub likes: Vec<Like>,
    pub followings: Vec<Following>,
}
