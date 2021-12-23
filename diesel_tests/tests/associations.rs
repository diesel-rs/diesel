use crate::schema::*;
use diesel::*;

#[test]
fn one_to_many_returns_query_source_for_association() {
    let (mut connection, sean, tess, _) = conn_with_test_data();

    let seans_posts = posts::table
        .filter(posts::user_id.eq(sean.id))
        .load::<Post>(&mut connection)
        .unwrap();
    let tess_posts = posts::table
        .filter(posts::user_id.eq(tess.id))
        .load::<Post>(&mut connection)
        .unwrap();

    let found_posts: Vec<_> = Post::belonging_to(&sean).load(&mut connection).unwrap();
    assert_eq!(seans_posts, found_posts);

    let found_posts: Vec<_> = Post::belonging_to(&tess).load(&mut connection).unwrap();
    assert_eq!(tess_posts, found_posts);
}

#[test]
fn eager_loading_associations_for_multiple_records() {
    let (mut connection, sean, tess, _) = conn_with_test_data();

    let users = vec![sean.clone(), tess.clone()];
    let posts = Post::belonging_to(&users)
        .load::<Post>(&mut connection)
        .unwrap()
        .grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    let seans_posts = Post::belonging_to(&sean).load(&mut connection).unwrap();
    let tess_posts = Post::belonging_to(&tess).load(&mut connection).unwrap();
    let expected_data = vec![(sean, seans_posts), (tess, tess_posts)];
    assert_eq!(expected_data, users_and_posts);
}

#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
mod eager_loading_with_string_keys {
    use crate::schema::{connection, drop_table_cascade};
    use diesel::connection::SimpleConnection;
    use diesel::*;

    table! { users { id -> Text, } }
    table! { posts { id -> Text, user_id -> Text, } }
    allow_tables_to_appear_in_same_query!(users, posts);

    #[derive(Queryable, Identifiable, Debug, PartialEq, Clone)]
    pub struct User {
        id: String,
    }

    #[derive(Queryable, Identifiable, Debug, PartialEq, Clone, Associations)]
    #[diesel(belongs_to(User))]
    pub struct Post {
        id: String,
        user_id: String,
    }

    #[test]
    fn eager_loading_associations_for_multiple_records() {
        let connection = &mut connection();
        drop_table_cascade(connection, "users");
        drop_table_cascade(connection, "posts");
        drop_table_cascade(connection, "fk_doesnt_reference_pk");
        connection
            .batch_execute(
                r#"
            CREATE TABLE users (id TEXT PRIMARY KEY NOT NULL);
            CREATE TABLE posts (id TEXT PRIMARY KEY NOT NULL, user_id TEXT NOT NULL);
            INSERT INTO users (id) VALUES ('Sean'), ('Tess');
            INSERT INTO posts (id, user_id) VALUES ('Hello', 'Sean'), ('World', 'Sean'), ('Hello 2', 'Tess');
        "#,
            )
            .unwrap();
        let sean = User { id: "Sean".into() };
        let tess = User { id: "Tess".into() };

        let users = vec![sean.clone(), tess.clone()];
        let posts = Post::belonging_to(&users)
            .load::<Post>(connection)
            .unwrap()
            .grouped_by(&users);
        let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

        let seans_posts = Post::belonging_to(&sean).load(connection).unwrap();
        let tess_posts = Post::belonging_to(&tess).load(connection).unwrap();
        let expected_data = vec![(sean, seans_posts), (tess, tess_posts)];
        assert_eq!(expected_data, users_and_posts);
    }
}

#[test]
fn grouping_associations_maintains_ordering() {
    let (mut connection, sean, tess, _) = conn_with_test_data();

    let users = vec![sean.clone(), tess.clone()];
    let posts = Post::belonging_to(&users)
        .order(posts::title.desc())
        .load::<Post>(&mut connection)
        .unwrap()
        .grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    let seans_posts = Post::belonging_to(&sean)
        .order(posts::title.desc())
        .load(&mut connection)
        .unwrap();
    let tess_posts = Post::belonging_to(&tess)
        .order(posts::title.desc())
        .load(&mut connection)
        .unwrap();
    let expected_data = vec![(sean.clone(), seans_posts), (tess.clone(), tess_posts)];
    assert_eq!(expected_data, users_and_posts);

    // Test when sorted manually
    let users = vec![sean.clone(), tess.clone()];
    let mut posts = Post::belonging_to(&users)
        .load::<Post>(&mut connection)
        .unwrap();
    posts.sort_by(|a, b| b.title.cmp(&a.title));
    let posts = posts.grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    assert_eq!(expected_data, users_and_posts);
}

#[test]
fn associations_can_be_grouped_multiple_levels_deep() {
    // I'm manually defining the data rather than loading from the database here,
    // as it makes the tests *way* more readable if I omit setup here. This is
    // the equivalent to.
    //
    // ```rust
    // let users = users.load::<User>().unwrap();
    // let posts = Post::belonging_to(&users).load::<Post>().unwrap();
    // let comments = Comment::belonging_to(&users).load::<Comment>().unwrap();
    // ```
    let users = vec![User::new(1, "Sean"), User::new(2, "Tess")];
    let posts = vec![
        Post::new(1, 1, "Hello", None),
        Post::new(2, 2, "World", None),
        Post::new(3, 1, "Hello 2", None),
    ];
    let comments = vec![
        Comment::new(1, 3, "LOL"),
        Comment::new(2, 1, "UR dumb"),
        Comment::new(3, 3, "Funny"),
    ]; // Never read the comments

    let expected_data = vec![
        (
            users[0].clone(),
            vec![
                (posts[0].clone(), vec![comments[1].clone()]),
                (
                    posts[2].clone(),
                    vec![comments[0].clone(), comments[2].clone()],
                ),
            ],
        ),
        (users[1].clone(), vec![(posts[1].clone(), vec![])]),
    ];

    let comments = comments.grouped_by(&posts);
    let posts_and_comments = posts.into_iter().zip(comments).grouped_by(&users);
    let data = users
        .into_iter()
        .zip(posts_and_comments)
        .collect::<Vec<_>>();

    assert_eq!(expected_data, data);
}

#[test]
fn self_referencing_associations() {
    #[derive(Insertable, Queryable, Associations, Identifiable, Debug, Clone, Copy, PartialEq)]
    #[diesel(table_name = trees)]
    #[diesel(belongs_to(Tree, foreign_key = parent_id))]
    struct Tree {
        id: i32,
        parent_id: Option<i32>,
    }

    let conn = &mut connection();
    let test_data = vec![
        Tree {
            id: 1,
            parent_id: None,
        },
        Tree {
            id: 2,
            parent_id: None,
        },
        Tree {
            id: 3,
            parent_id: Some(1),
        },
        Tree {
            id: 4,
            parent_id: Some(2),
        },
        Tree {
            id: 5,
            parent_id: Some(1),
        },
    ];
    insert_into(trees::table)
        .values(&test_data)
        .execute(conn)
        .unwrap();

    let parents = trees::table
        .filter(trees::parent_id.is_null())
        .load::<Tree>(conn)
        .unwrap();
    let children = Tree::belonging_to(&parents).load::<Tree>(conn).unwrap();
    let children = children.grouped_by(&parents);
    let data = parents.into_iter().zip(children).collect::<Vec<_>>();

    let expected_data = vec![
        (test_data[0], vec![test_data[2], test_data[4]]),
        (test_data[1], vec![test_data[3]]),
    ];
    assert_eq!(expected_data, data);
}

fn conn_with_test_data() -> (TestConnection, User, User, User) {
    let mut connection = connection_with_sean_and_tess_in_users_table();
    insert_into(users::table)
        .values(&NewUser::new("Jim", None))
        .execute(&mut connection)
        .unwrap();

    let sean = find_user_by_name("Sean", &mut connection);
    let tess = find_user_by_name("Tess", &mut connection);
    let jim = find_user_by_name("Jim", &mut connection);
    let new_posts = vec![sean.new_post("Hello", None), sean.new_post("World", None)];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&mut connection)
        .unwrap();
    let new_posts = vec![
        tess.new_post("Hello 2", None),
        tess.new_post("World 2", None),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&mut connection)
        .unwrap();
    let new_posts = vec![jim.new_post("Hello 3", None), jim.new_post("World 3", None)];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&mut connection)
        .unwrap();

    (connection, sean, tess, jim)
}

#[test]
#[cfg(not(feature = "mysql"))] // FIXME: Figure out how to handle tests that modify schema
fn custom_foreign_key() {
    use diesel::connection::SimpleConnection;
    use diesel::*;

    table! {
        users1 {
            id -> Integer,
            name -> Text,
        }
    }

    table! {
        posts1 {
            id -> Integer,
            belongs_to_user -> Integer,
            title -> Text,
        }
    }

    allow_tables_to_appear_in_same_query!(users1, posts1);

    #[derive(Clone, Debug, PartialEq, Identifiable, Queryable)]
    #[diesel(table_name = users1)]
    pub struct User {
        id: i32,
        name: String,
    }

    #[derive(Clone, Debug, PartialEq, Associations, Identifiable, Queryable)]
    #[diesel(belongs_to(User, foreign_key = belongs_to_user))]
    #[diesel(table_name = posts1)]
    pub struct Post {
        id: i32,
        belongs_to_user: i32,
        title: String,
    }

    joinable!(posts1 -> users1(belongs_to_user));
    let connection = &mut connection();
    connection
        .batch_execute(
            r#"
            CREATE TABLE users1 (id SERIAL PRIMARY KEY,
                                name TEXT NOT NULL);
            CREATE TABLE posts1 (id SERIAL PRIMARY KEY,
                                belongs_to_user INTEGER NOT NULL,
                                title TEXT NOT NULL);
            INSERT INTO users1 (id, name) VALUES (1, 'Sean'), (2, 'Tess');
            INSERT INTO posts1 (id, belongs_to_user, title) VALUES
                   (1, 1, 'Hello'),
                   (2, 2, 'World'),
                   (3, 1, 'Hello 2');
        "#,
        )
        .unwrap();

    let sean = User {
        id: 1,
        name: "Sean".into(),
    };
    let tess = User {
        id: 2,
        name: "Tess".into(),
    };
    let post1 = Post {
        id: 1,
        belongs_to_user: 1,
        title: "Hello".into(),
    };
    let post2 = Post {
        id: 2,
        belongs_to_user: 2,
        title: "World".into(),
    };
    let post3 = Post {
        id: 3,
        belongs_to_user: 1,
        title: "Hello 2".into(),
    };

    assert_eq!(
        Post::belonging_to(&sean).load(connection),
        Ok(vec![post1.clone(), post3.clone()])
    );

    assert_eq!(
        users1::table.inner_join(posts1::table).load(connection),
        Ok(vec![(sean.clone(), post1), (tess, post2), (sean, post3)])
    );
}
