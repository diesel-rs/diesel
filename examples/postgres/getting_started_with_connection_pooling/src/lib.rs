pub mod models;
pub mod schema;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::r2d2::PooledConnection;
use dotenvy::dotenv;
use std::env;

use self::models::{NewPost, Post};

pub type PostgresPool = Pool<ConnectionManager<PgConnection>>;

pub fn establish_connection() -> PooledConnection<ConnectionManager<PgConnection>> {
    get_pg_pool().get().unwrap()
}

pub fn get_pg_pool() -> PostgresPool {
    dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(url);
    // Refer to the `r2d2` documentation for more methods to use on Pool build
    Pool::builder()
        .max_size(2)
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool")
}

pub fn create_post(conn: &mut PgConnection, title: &str, body: &str) -> Post {
    use crate::schema::posts;

    let new_post = NewPost { title, body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .get_result(conn)
        .expect("Error saving new post")
}
