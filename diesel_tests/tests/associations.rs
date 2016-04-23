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
