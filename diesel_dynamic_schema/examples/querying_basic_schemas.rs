//! Example: querying basic schemas
//!
//! To run this:
//!
//! ```sh
//! cargo run --example querying_basic_schemas --features="diesel/sqlite"
//! ```
extern crate diesel;
extern crate diesel_dynamic_schema;

use diesel::sql_types::Integer;
use diesel::sqlite::SqliteConnection;
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

    // Use typical Diesel syntax to get some data.
    let ids = users.select(id).load::<i32>(conn);

    // Print the results.
    // The `ids` are type `std::result::Result<std::vec::Vec<i32>, diesel::result::Error>`.
    let ids = ids.unwrap();
    for x in ids {
        println!("user id:{}", x);
    }
}
