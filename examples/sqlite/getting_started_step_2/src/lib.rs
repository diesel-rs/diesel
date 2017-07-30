#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate dotenv;

pub mod schema;
pub mod models;

use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use self::models::NewPost;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
    .expect(&format!("Error connecting to {}",
    database_url))
}

pub fn create_post(conn: &SqliteConnection, title: &str, body: &str) -> usize {
    use schema::posts;

    let new_post = NewPost {
        title: title,
        body: body,
    };

    diesel::insert(&new_post)
        .into(posts::table)
        .execute(conn)
        .expect("Error saving new post")
}
