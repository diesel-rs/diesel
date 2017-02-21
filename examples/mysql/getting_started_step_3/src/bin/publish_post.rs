extern crate diesel_demo_step_3_mysql;
extern crate diesel;

use diesel::prelude::*;
use diesel_demo_step_3_mysql::*;
use self::models::Post;
use std::env::args;

fn main() {
    use self::schema::posts::dsl::{posts, published};

    let id = args().nth(1).expect("publish_post requires a post id")
        .parse::<i32>().expect("Invalid ID");
    let connection = establish_connection();

    let post: Post = posts.find(id)
        .first(&connection)
        .expect(&format!("Unable to find post {}", id));

    diesel::update(posts.find(id))
        .set(published.eq(true))
        .execute(&connection)
        .unwrap();

    println!("Published post {}", post.title);
}
