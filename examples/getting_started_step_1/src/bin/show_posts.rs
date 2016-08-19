extern crate diesel_demo_step_1;
extern crate diesel;

use diesel_demo_step_1::*;
use diesel_demo_step_1::models::*;
use diesel::prelude::*;

fn main() {
    use diesel_demo_step_1::schema::posts::dsl::*;

    let connection = establish_connection();
    let results = posts.filter(published.eq(true))
        .limit(5)
        .load::<Post>(&connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("-----------\n");
        println!("{}", post.body);
    }
}
