use schema::*;
use diesel::*;

#[test]
fn one_to_many_returns_query_source_for_association() {
    let (connection, sean, tess, _) = conn_with_test_data();

    let seans_posts = posts::table.filter(posts::user_id.eq(sean.id))
        .load::<Post>(&connection)
        .unwrap();
    let tess_posts = posts::table.filter(posts::user_id.eq(tess.id))
        .load::<Post>(&connection)
        .unwrap();

    let found_posts: Vec<_> = Post::belonging_to(&sean).load(&connection).unwrap();
    assert_eq!(seans_posts, found_posts);

    let found_posts: Vec<_> = Post::belonging_to(&tess).load(&connection).unwrap();
    assert_eq!(tess_posts, found_posts);
}

#[test]
fn eager_loading_associations_for_multiple_records() {
    let (connection, sean, tess, _) = conn_with_test_data();

    let users = vec![sean.clone(), tess.clone()];
    let posts = Post::belonging_to(&users).load::<Post>(&connection).unwrap()
        .grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    let seans_posts = Post::belonging_to(&sean).load(&connection).unwrap();
    let tess_posts = Post::belonging_to(&tess).load(&connection).unwrap();
    let expected_data = vec![(sean, seans_posts), (tess, tess_posts)];
    assert_eq!(expected_data, users_and_posts);
}

mod eager_loading_with_string_keys {
    use diesel::*;
    use diesel::connection::SimpleConnection;
    use schema::connection;

    table! { users { id -> Text, } }
    table! { posts { id -> Text, user_id -> Text, } }

    #[derive(Queryable, Identifiable, Debug, PartialEq, Clone)]
    pub struct User {
        id: String,
    }

    #[derive(Queryable, Identifiable, Debug, PartialEq, Clone)]
    #[belongs_to(User)]
    pub struct Post {
        id: String,
        user_id: String,
    }

    #[test]
    fn eager_loading_associations_for_multiple_records() {
        let connection = connection();
        connection.batch_execute(r#"
            DROP TABLE users;
            DROP TABLE posts;
            CREATE TABLE users (id TEXT PRIMARY KEY NOT NULL);
            CREATE TABLE posts (id TEXT PRIMARY KEY NOT NULL, user_id TEXT NOT NULL);
            INSERT INTO users (id) VALUES ('Sean'), ('Tess');
            INSERT INTO posts (id, user_id) VALUES ('Hello', 'Sean'), ('World', 'Sean'), ('Hello 2', 'Tess');
        "#).unwrap();
        let sean = User { id: "Sean".into() };
        let tess = User { id: "Tess".into() };

        let users = vec![sean.clone(), tess.clone()];
        let posts = Post::belonging_to(&users).load::<Post>(&connection).unwrap()
            .grouped_by(&users);
        let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

        let seans_posts = Post::belonging_to(&sean).load(&connection).unwrap();
        let tess_posts = Post::belonging_to(&tess).load(&connection).unwrap();
        let expected_data = vec![(sean, seans_posts), (tess, tess_posts)];
        assert_eq!(expected_data, users_and_posts);
    }
}

#[test]
fn grouping_associations_maintains_ordering() {
    let (connection, sean, tess, _) = conn_with_test_data();

    let users = vec![sean.clone(), tess.clone()];
    let posts = Post::belonging_to(&users)
        .order(posts::title.desc())
        .load::<Post>(&connection).unwrap()
        .grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    let seans_posts = Post::belonging_to(&sean).order(posts::title.desc()).load(&connection).unwrap();
    let tess_posts = Post::belonging_to(&tess).order(posts::title.desc()).load(&connection).unwrap();
    let expected_data = vec![(sean.clone(), seans_posts), (tess.clone(), tess_posts)];
    assert_eq!(expected_data, users_and_posts);

    // Test when sorted manually
    let users = vec![sean.clone(), tess.clone()];
    let mut posts = Post::belonging_to(&users)
        .load::<Post>(&connection).unwrap();
    posts.sort_by(|a, b| b.title.cmp(&a.title));
    let posts = posts.grouped_by(&users);
    let users_and_posts = users.into_iter().zip(posts).collect::<Vec<_>>();

    assert_eq!(expected_data, users_and_posts);
}

fn associations_can_be_grouped_multiple_levels_deep() {
    // I'm manually defining the data rather than loding from the database here,
    // as it makes the tests *way* more readable if I omit setup here. This is
    // the equivalent to.
    //
    // ```rust
    // let users = users.load::<User>().unwrap();
    // let posts = Post::belonging_to(&users).load::<Post>().unwrap();
    // let comments = Comment::belonging_to(&users).load::<Comment>().unwrap();
    // ```
    let users = vec![User::new(1, "Sean"), User::new(2, "Tess")];
    let posts = vec![Post::new(1, 1, "Hello", None), Post::new(2, 2, "World", None), Post::new(3, 1, "Hello 2", None)];
    let comments = vec![Comment::new(1, 3, "LOL"), Comment::new(2, 1, "UR dumb"), Comment::new(3, 3, "Funny")]; // Never read the comments

    let expected_data = vec![
        (users[0].clone(), vec![(posts[0].clone(), vec![comments[1].clone()]), (posts[2].clone(), vec![comments[0].clone(), comments[2].clone()])]),
        (users[1].clone(), vec![(posts[1].clone(), vec![])]),
    ];

    let comments = comments.grouped_by(&posts);
    let posts_and_comments = posts.into_iter().zip(comments).grouped_by(&users);
    let data = users.into_iter().zip(posts_and_comments).collect::<Vec<_>>();

    assert_eq!(expected_data, data);
}

fn conn_with_test_data() -> (TestConnection, User, User, User) {
    let connection = connection_with_sean_and_tess_in_users_table();
    insert(&NewUser::new("Jim", None)).into(users::table).execute(&connection).unwrap();

    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);
    let jim = find_user_by_name("Jim", &connection);
    let new_posts = vec![sean.new_post("Hello", None), sean.new_post("World", None)];
    batch_insert(&new_posts, posts::table, &connection);
    let new_posts = vec![tess.new_post("Hello 2", None), tess.new_post("World 2", None)];
    batch_insert(&new_posts, posts::table, &connection);
    let new_posts = vec![jim.new_post("Hello 3", None), jim.new_post("World 3", None)];
    batch_insert(&new_posts, posts::table, &connection);

    (connection, sean, tess, jim)
}
