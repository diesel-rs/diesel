use super::schema::*;
use diesel::*;

#[test]
fn selecting_basic_data() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![
        User {
            id: 1,
            name: "Sean".to_string(),
            hair_color: None,
        },
        User {
            id: 2,
            name: "Tess".to_string(),
            hair_color: None,
        },
    ];
    let actual_data: Vec<_> = users
        .select(User::as_select())
        .order(id)
        .load(connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
fn with_safe_select() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(connection)
        .unwrap();

    let select_name = users.select(UserName::as_select());
    let names: Vec<UserName> = select_name.load(connection).unwrap();

    assert_eq!(vec![UserName::new("Sean"), UserName::new("Tess")], names);
}

table! {
    select {
        id -> Integer,
        join -> Integer,
    }
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_and_tables_with_reserved_names() {
    use crate::schema_dsl::*;
    let connection = &mut connection();
    create_table(
        "select",
        (
            integer("id").primary_key().auto_increment(),
            integer("join").not_null(),
        ),
    )
    .execute(connection)
    .unwrap();
    diesel::sql_query("INSERT INTO \"select\" (\"join\") VALUES (1), (2), (3)")
        .execute(connection)
        .unwrap();

    #[derive(Debug, PartialEq, Queryable, Selectable)]
    #[diesel(table_name = select)]
    struct Select {
        join: i32,
    }

    let expected_data = vec![1, 2, 3]
        .into_iter()
        .map(|join| Select { join })
        .collect::<Vec<_>>();
    let actual_data: Vec<Select> = select::table
        .select(Select::as_select())
        .load(connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn selecting_columns_with_different_definition_order() {
    use crate::schema_dsl::*;
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("hair_color"),
            string("name").not_null(),
        ),
    )
    .execute(connection)
    .unwrap();
    let expected_user = User::with_hair_color(1, "Sean", "black");
    insert_into(users::table)
        .values(&NewUser::new("Sean", Some("black")))
        .execute(connection)
        .unwrap();
    let user_from_select = users::table.select(User::as_select()).first(connection);

    assert_eq!(Ok(&expected_user), user_from_select.as_ref());
}

#[test]
fn selection_using_subselect() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let ids: Vec<i32> = users::table
        .select(users::id)
        .order(users::id)
        .load(connection)
        .unwrap();
    let query = format!(
        "INSERT INTO posts (user_id, title) VALUES ({}, 'Hello'), ({}, 'World')",
        ids[0], ids[1]
    );
    diesel::sql_query(query).execute(connection).unwrap();

    #[derive(Debug, PartialEq, Queryable, Selectable)]
    struct Post(#[diesel(column_name = title)] String);

    let users = users::table
        .filter(users::name.eq("Sean"))
        .select(users::id);
    let data = posts::table
        .select(Post::as_select())
        .filter(posts::user_id.eq_any(users))
        .load(connection)
        .unwrap();

    assert_eq!(vec![Post("Hello".to_string())], data);
}

#[test]
fn select_can_be_called_on_query_that_is_valid_subselect_but_invalid_query() {
    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", connection);
    let tess = find_user_by_name("Tess", connection);
    insert_into(posts::table)
        .values(&vec![
            tess.new_post("Tess", None),
            sean.new_post("Hi", None),
        ])
        .execute(connection)
        .unwrap();

    let invalid_query_but_valid_subselect = posts::table
        .select(posts::user_id)
        .filter(posts::title.eq(users::name));
    let users_with_post_using_name_as_title = users::table
        .select(User::as_select())
        .filter(users::id.eq_any(invalid_query_but_valid_subselect))
        .load(connection);

    assert_eq!(Ok(vec![tess]), users_with_post_using_name_as_title);
}

#[test]
fn selecting_multiple_aggregate_expressions_without_group_by() {
    use self::users::dsl::*;
    use diesel::dsl::{count_star, max, CountStar};
    use diesel::helper_types::max;

    #[derive(Queryable)]
    struct CountAndMax {
        count: i64,
        max_name: Option<String>,
    }
    impl<DB> Selectable<DB> for CountAndMax
    where
        DB: diesel::backend::Backend,
    {
        type SelectExpression = (CountStar, max<name>);

        fn construct_selection() -> Self::SelectExpression {
            (count_star(), max(name))
        }
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();
    let CountAndMax { count, max_name } = users
        .select(CountAndMax::as_select())
        .get_result(connection)
        .unwrap();

    assert_eq!(2, count);
    assert_eq!(Some(String::from("Tess")), max_name);
}

#[test]
fn mixed_selectable_and_plain_select() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let expected_data = vec![
        (
            User {
                id: 1,
                name: "Sean".to_string(),
                hair_color: None,
            },
            "Sean".to_string(),
        ),
        (
            User {
                id: 2,
                name: "Tess".to_string(),
                hair_color: None,
            },
            "Tess".to_string(),
        ),
    ];
    let actual_data: Vec<_> = users
        .select((User::as_select(), name))
        .order(id)
        .load(connection)
        .unwrap();
    assert_eq!(expected_data, actual_data);
}

// The following tests are duplicates from tests in joins.rs
// They are used to verify that selectable behaves equivalent to the corresponding
// raw select
#[test]
fn selecting_parent_child_grandchild() {
    use crate::joins::TestData;

    let (mut connection, test_data) =
        crate::joins::connection_with_fixture_data_for_multitable_joins();
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
        .select(<(User, (Post, Comment)) as SelectableHelper<_>>::as_select())
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
        .select(<(User, (Post, Comment)) as SelectableHelper<_>>::as_select())
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), (posts[0].clone(), comments[0].clone())),
        (sean.clone(), (posts[0].clone(), comments[2].clone())),
    ];
    assert_eq!(Ok(expected), data);

    let data = users::table
        .inner_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .select(<(User, (Post, Option<Comment>)) as SelectableHelper<_>>::as_select())
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
        .select(<(User, Option<(Post, Option<Comment>)>) as SelectableHelper<_>>::as_select())
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
        .select(<(User, Option<(Post, Comment)>) as SelectableHelper<_>>::as_select())
        .load(&mut connection);
    let expected = vec![
        (sean.clone(), Some((posts[0].clone(), comments[0].clone()))),
        (sean.clone(), Some((posts[0].clone(), comments[2].clone()))),
        (sean, Some((posts[2].clone(), comments[1].clone()))),
        (tess, None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_parent_child_grandchild_nested() {
    use crate::joins::TestData;

    let (mut connection, test_data) =
        crate::joins::connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        tess,
        posts,
        comments,
        ..
    } = test_data;

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = users)]
    struct User1 {
        id: i32,
        name: String,
        hair_color: Option<String>,
        #[diesel(embed)]
        post: Post1,
    }

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = posts)]
    struct Post1 {
        id: i32,
        user_id: i32,
        title: String,
        body: Option<String>,
        #[diesel(embed)]
        comment: Comment,
    }

    let data = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .select(User1::as_select())
        .load(&mut connection);
    let expected = vec![
        User1 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post1 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: comments[0].clone(),
            },
        },
        User1 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post1 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: comments[2].clone(),
            },
        },
        User1 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post1 {
                id: posts[2].id,
                user_id: posts[2].user_id,
                title: posts[2].title.clone(),
                body: posts[2].body.clone(),
                comment: comments[1].clone(),
            },
        },
    ];
    assert_eq!(Ok(expected), data);

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = users)]
    struct User2 {
        id: i32,
        name: String,
        hair_color: Option<String>,
        #[diesel(embed)]
        post: Post2,
    }

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = posts)]
    struct Post2 {
        id: i32,
        user_id: i32,
        title: String,
        body: Option<String>,
        #[diesel(embed)]
        comment: Option<Comment>,
    }

    let data = users::table
        .inner_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .select(User2::as_select())
        .load(&mut connection);
    let expected = vec![
        User2 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post2 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: Some(comments[0].clone()),
            },
        },
        User2 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post2 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: Some(comments[2].clone()),
            },
        },
        User2 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Post2 {
                id: posts[2].id,
                user_id: posts[2].user_id,
                title: posts[2].title.clone(),
                body: posts[2].body.clone(),
                comment: Some(comments[1].clone()),
            },
        },
        User2 {
            id: tess.id,
            name: tess.name.clone(),
            hair_color: tess.hair_color.clone(),
            post: Post2 {
                id: posts[1].id,
                user_id: posts[1].user_id,
                title: posts[1].title.clone(),
                body: posts[1].body.clone(),
                comment: None,
            },
        },
    ];
    assert_eq!(Ok(expected), data);

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = users)]
    struct User3 {
        id: i32,
        name: String,
        hair_color: Option<String>,
        #[diesel(embed)]
        post: Option<Post3>,
    }

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = posts)]
    struct Post3 {
        id: i32,
        user_id: i32,
        title: String,
        body: Option<String>,
        #[diesel(embed)]
        comment: Option<Comment>,
    }

    let data = users::table
        .left_outer_join(posts::table.left_outer_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .select(User3::as_select())
        .load(&mut connection);
    let expected = vec![
        User3 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Some(Post3 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: Some(comments[0].clone()),
            }),
        },
        User3 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Some(Post3 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: Some(comments[2].clone()),
            }),
        },
        User3 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Some(Post3 {
                id: posts[2].id,
                user_id: posts[2].user_id,
                title: posts[2].title.clone(),
                body: posts[2].body.clone(),
                comment: Some(comments[1].clone()),
            }),
        },
        User3 {
            id: tess.id,
            name: tess.name.clone(),
            hair_color: tess.hair_color.clone(),
            post: Some(Post3 {
                id: posts[1].id,
                user_id: posts[1].user_id,
                title: posts[1].title.clone(),
                body: posts[1].body.clone(),
                comment: None,
            }),
        },
    ];
    assert_eq!(Ok(expected), data);

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = users)]
    struct User4 {
        id: i32,
        name: String,
        hair_color: Option<String>,
        #[diesel(embed)]
        post: Option<Post4>,
    }

    #[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
    #[diesel(table_name = posts)]
    struct Post4 {
        id: i32,
        user_id: i32,
        title: String,
        body: Option<String>,
        #[diesel(embed)]
        comment: Comment,
    }

    let data = users::table
        .left_outer_join(posts::table.inner_join(comments::table))
        .order((users::id, posts::id, comments::id))
        .select(User4::as_select())
        .load(&mut connection);
    let expected = vec![
        User4 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Some(Post4 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: comments[0].clone(),
            }),
        },
        User4 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color.clone(),
            post: Some(Post4 {
                id: posts[0].id,
                user_id: posts[0].user_id,
                title: posts[0].title.clone(),
                body: posts[0].body.clone(),
                comment: comments[2].clone(),
            }),
        },
        User4 {
            id: sean.id,
            name: sean.name.clone(),
            hair_color: sean.hair_color,
            post: Some(Post4 {
                id: posts[2].id,
                user_id: posts[2].user_id,
                title: posts[2].title.clone(),
                body: posts[2].body.clone(),
                comment: comments[1].clone(),
            }),
        },
        User4 {
            id: tess.id,
            name: tess.name.clone(),
            hair_color: tess.hair_color,
            post: None,
        },
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_grandchild_child_parent() {
    use crate::joins::TestData;
    let (mut connection, test_data) =
        crate::joins::connection_with_fixture_data_for_multitable_joins();
    let TestData {
        sean,
        posts,
        comments,
        ..
    } = test_data;

    let data = comments::table
        .inner_join(posts::table.inner_join(users::table))
        .order((users::id, posts::id, comments::id))
        .select(<(Comment, (Post, User)) as SelectableHelper<_>>::as_select())
        .load(&mut connection);
    let expected = vec![
        (comments[0].clone(), (posts[0].clone(), sean.clone())),
        (comments[2].clone(), (posts[0].clone(), sean.clone())),
        (comments[1].clone(), (posts[2].clone(), sean)),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn selecting_crazy_nested_joins() {
    use crate::joins::TestData;
    let (mut connection, test_data) =
        crate::joins::connection_with_fixture_data_for_multitable_joins();
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
        .select(<(
            User,
            (Post, Option<(Comment, Option<Like>)>, Option<Following>),
        ) as SelectableHelper<_>>::as_select())
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
        .select(<(
            User,
            (Post, Option<(Comment, Option<Like>)>),
            Option<Following>,
        ) as SelectableHelper<_>>::as_select())
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
