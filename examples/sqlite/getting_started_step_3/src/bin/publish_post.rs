use diesel::prelude::*;
use getting_started_step_3_sqlite::models::Post;
use getting_started_step_3_sqlite::*;
use std::env::args;

fn main() {
    use getting_started_step_3_sqlite::schema::posts::dsl::{posts, published};

    let id = args()
        .nth(1)
        .expect("publish_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");
    let connection = &mut establish_connection();

    let post = diesel::update(posts.find(id))
        .set(published.eq(true))
        .returning(Post::as_returning())
        .get_result(connection)
        .unwrap();

    println!("Published post {}", post.title);
}
