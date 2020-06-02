use diesel::prelude::*;
use diesel_demo_step_3_sqlite::models::*;
use diesel_demo_step_3_sqlite::*;

fn main() {
    use diesel_demo_step_3_sqlite::schema::posts::dsl::*;

    let connection = establish_connection();
    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .load::<Post>(&connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("----------\n");
        println!("{}", post.body);
    }
}
