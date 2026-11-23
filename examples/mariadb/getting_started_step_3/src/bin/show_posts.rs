use self::models::*;
use diesel::prelude::*;
use getting_started_step_3_mariadb::*;

fn main() {
    use self::schema::posts::dsl::*;

    let connection = &mut establish_connection();
    let results = Post::query()
        .filter(published.eq(true))
        .limit(5)
        .load(connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("-----------\n");
        println!("{}", post.body);
    }
}
