use diesel::prelude::*;
use getting_started_step_3_sqlite::*;
use std::env::args;

fn main() {
    use getting_started_step_3_sqlite::schema::posts::dsl::*;

    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{target}%");

    let connection = &mut establish_connection();
    let num_deleted = diesel::delete(posts.filter(title.like(pattern)))
        .execute(connection)
        .expect("Error deleting posts");

    println!("Deleted {num_deleted} posts");
}
