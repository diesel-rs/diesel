use schema::*;
use yaqb::*;

#[test]
fn one_to_many_returns_query_source_for_association() {
    let connection = connection_with_sean_and_tess_in_users_table();
    setup_posts_table(&connection);

    let sean: User = connection.find(users::table, 1).unwrap().unwrap();
    let tess: User = connection.find(users::table, 2).unwrap().unwrap();
    let seans_posts: Vec<Post> = connection.insert(&posts::table, &vec![
        sean.new_post("Hello", None), sean.new_post("World", None),
    ]).unwrap().collect();
    let tess_posts: Vec<Post> = connection.insert(&posts::table, &vec![
        tess.new_post("Hello 2", None), tess.new_post("World 2", None),
    ]).unwrap().collect();

    let found_posts: Vec<_> = Post::belonging_to(&sean).load(&connection).unwrap().collect();
    assert_eq!(seans_posts, found_posts);

    let found_posts: Vec<_> = Post::belonging_to(&tess).load(&connection).unwrap().collect();
    assert_eq!(tess_posts, found_posts);
}
