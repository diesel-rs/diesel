//! Example: querying multiple types
//!
//! To run this:
//!
//! ```sh
//! cargo run --example querying_multiple_types --features="diesel/sqlite"
//! ```
extern crate diesel;
extern crate diesel_dynamic_schema;

use diesel::sql_types::{Integer, Text};
use diesel::*;
use diesel_dynamic_schema::table;

fn main() {
    // Create a connection; we are using a simple Sqlite memory database.
    let conn = &mut SqliteConnection::establish(":memory:").unwrap();

    // Create some example data by using typical SQL statements.
    sql_query("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)")
        .execute(conn)
        .unwrap();
    sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Tess')")
        .execute(conn)
        .unwrap();

    // Use diesel-dynamic-schema to create a table and a column.
    let users = table("users");
    let id = users.column::<Integer, _>("id");
    let name = users.column::<Text, _>("name");

    // Use typical Diesel syntax to get some data.
    let users = users.select((id, name)).load::<(i32, String)>(conn);

    // Print the results.
    // The `users` are type `std::result::Result<std::vec::Vec<(i32, std::string::String)>, diesel::result::Error>`
    let users = users.unwrap();
    for (user_id, user_name) in users {
        println!("user id:{} name:{}", user_id, user_name);
    }
}
