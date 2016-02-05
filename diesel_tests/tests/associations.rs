use schema::*;
use diesel::*;

#[test]
fn one_to_many_returns_query_source_for_association() {
    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &connection);
    let tess = find_user_by_name("Tess", &connection);
    let new_posts = vec![sean.new_post("Hello", None), sean.new_post("World", None)];
    batch_insert(&new_posts, posts::table, &connection);
    let new_posts = vec![tess.new_post("Hello 2", None), tess.new_post("World 2", None)];
    batch_insert(&new_posts, posts::table, &connection);
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
