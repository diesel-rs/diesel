extern crate diesel_demo_step_3;
extern crate diesel;

use diesel::prelude::*;
use diesel_demo_step_3::*;
use std::env::args;

fn main() {
    use diesel_demo_step_3::schema::posts::dsl::*;

    let target = args().nth(1).expect("Expected a target to match against");
    let pattern = format!("%{}%", target);

    let connection = establish_connection();
    let num_deleted = diesel::delete(posts.filter(title.like(pattern)))
        .execute(&connection)
        .expect("Error deleting posts");

    println!("Deleted {} posts", num_deleted);
}
