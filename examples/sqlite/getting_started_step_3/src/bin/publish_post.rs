use diesel::prelude::*;
use diesel_demo_step_3_sqlite::models::Post;
use diesel_demo_step_3_sqlite::*;
use std::env::args;

fn main() {
    use diesel_demo_step_3_sqlite::schema::posts::dsl::{posts, published};

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
