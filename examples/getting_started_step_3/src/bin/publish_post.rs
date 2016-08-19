extern crate diesel_demo_step_3;
extern crate diesel;

use diesel::prelude::*;
use diesel_demo_step_3::*;
use diesel_demo_step_3::models::Post;
use std::env::args;

fn main() {
    use diesel_demo_step_3::schema::posts::dsl::{posts, published};

    let id = args().nth(1).expect("publish_post requires a post id")
        .parse::<i32>().expect("Invalid ID");
    let connection = establish_connection();

    let post = diesel::update(posts.find(id))
        .set(published.eq(true))
        .get_result::<Post>(&connection)
        .expect(&format!("Unable to find post {}", id));
    println!("Published post {}", post.title);
}
