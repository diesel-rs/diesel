use self::models::Post;
use diesel::prelude::*;
use diesel_demo_step_3_mysql::*;
use std::env::args;

fn main() {
    use self::schema::posts::dsl::{posts, id};

    let post_id = args()
        .nth(1)
        .expect("get_post requires a post id")
        .parse::<i32>()
        .expect("Invalid ID");
    
    let connection = &mut establish_connection();

    let post = connection
        .transaction(|connection| {
            let post = posts
                .find(id)
                .select(Post::as_select())
                .first(connection)
                .optional()?; // This allows for returning an Option<Post>, otherwise it will throw an error

            match post {
                Some(post) => Ok(post),
                None => Err(diesel::result::Error::NotFound),
            }
        })
        .unwrap_or_else(|_: diesel::result::Error| panic!("Unable to find post {}", post_id));

    println!("Post with id: {} has a title: {}", post.id, post.title);
}
