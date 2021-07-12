use self::models::*;
use diesel::prelude::*;
use diesel_demo_step_1_sqlite::*;

fn main() {
    use self::schema::posts::dsl::*;

    let connection = establish_connection();
    // let sql = r#"CREATE TABLE posts (
    //     id INTEGER NOT NULL PRIMARY KEY,
    //     title VARCHAR NOT NULL,
    //     body TEXT NOT NULL,
    //     published BOOLEAN NOT NULL DEFAULT 0
    //   )"#;
    // connection.execute(sql).unwrap();
    // let sql = r#"delete from posts where id = 1"#;
    // connection.execute(sql).unwrap();
    // let sql = r#"insert into posts (id,title,body,published)
    // values (2, 'test', 'testxxxx', true)"#;
    // connection.execute(sql).unwrap();
    let sql = r#"select id,title,body,published from posts "#;
    connection.execute(sql).unwrap();

    let results = posts
        .filter(published.eq(true))
        // .limit(1)
        .load::<Post>(&connection)
        .expect("Error loading posts");

    println!("Displaying {} posts", results.len());
    for post in results {
        println!("{}", post.title);
        println!("----------\n");
        println!("{}", post.body);
    }
}
