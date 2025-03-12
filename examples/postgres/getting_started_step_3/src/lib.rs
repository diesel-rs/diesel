pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

use self::models::{NewPost, Post};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("PG_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Failed to connect, error: {}", e))
}

pub fn create_post(conn: &mut PgConnection, title: &str, body: &str) -> Post {
    use crate::schema::posts;

    let new_post = NewPost { title, body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error saving new post")
}
