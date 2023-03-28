use self::models::Post;
use diesel::prelude::*;
use diesel_demo_step_3_mysql::*;
use std::env::args;

fn main() {
    use self::schema::posts::dsl::{posts, published};

    let id = args()
        .nth(1)
        .expect("publish_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");
    let connection = &mut establish_connection();

    let post = connection
        .transaction(|connection| {
            let post = posts.find(id).select(Post::as_select()).first(connection)?;

            diesel::update(posts.find(id))
                .set(published.eq(true))
                .execute(connection)?;
            Ok(post)
        })
        .unwrap_or_else(|_: diesel::result::Error| panic!("Unable to find post {}", id));

    println!("Published post {}", post.title);
}
