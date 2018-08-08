extern crate diesel;
extern crate diesel_demo_step_3_pg;

use self::models::Post;
use diesel::prelude::*;
use diesel_demo_step_3_pg::*;
use std::env::args;

fn main() {
    use self::schema::posts::dsl::{posts, published};

    let id = args()
        .nth(1)
        .expect("publish_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");
    let connection = establish_connection();

    let post = diesel::update(posts.find(id))
        .set(published.eq(true))
        .get_result::<Post>(&connection)
        .unwrap_or_else(|_| panic!("Unable to find post {}", id));
    println!("Published post {}", post.title);
}
