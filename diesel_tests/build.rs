#[cfg(not(feature = "unstable"))]
mod inner {
    extern crate syntex;
    extern crate diesel_codegen;
    extern crate dotenv_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();
        diesel_codegen::register(&mut registry);
        dotenv_codegen::register(&mut registry);

        let src = Path::new("tests/lib.in.rs");
        let dst = Path::new(&out_dir).join("lib.rs");

        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(feature = "unstable")]
mod inner {
    pub fn main() {}
}

extern crate diesel;
extern crate dotenv;
use diesel::*;
use dotenv::dotenv;

fn main() {
    dotenv().ok();
    let database_url = ::std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set to run tests");
    let connection = Connection::establish(&database_url).unwrap();
    setup_tables_for_schema(&connection);
    inner::main();
}

fn setup_tables_for_schema(connection: &Connection) {
    connection.execute("DROP TABLE IF EXISTS users").unwrap();
    connection.execute("DROP TABLE IF EXISTS posts").unwrap();
    connection.execute("DROP TABLE IF EXISTS comments").unwrap();
    connection.execute("CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        name VARCHAR NOT NULL,
        hair_color VARCHAR
    )").unwrap();
    connection.execute("CREATE TABLE posts (
        id SERIAL PRIMARY KEY,
        user_id INTEGER NOT NULL,
        title VARCHAR NOT NULL,
        body TEXT
    )").unwrap();
    connection.execute("CREATE TABLE comments (
        id SERIAL PRIMARY KEY,
        post_id INTEGER NOT NULL,
        text TEXT NOT NULL
    )").unwrap();
}
