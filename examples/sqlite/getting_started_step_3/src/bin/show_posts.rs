use diesel::prelude::*;
use getting_started_step_3_sqlite::models::*;
use getting_started_step_3_sqlite::*;

fn main() {
    use getting_started_step_3_sqlite::schema::posts::dsl::*;

    let connection = &mut establish_connection();
    let results = posts
        .filter(published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("----------\n");
        println!("{}", post.body);
    }
}
