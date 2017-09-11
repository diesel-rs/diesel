#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let conn = SqliteConnection::establish("").unwrap();
    users::table
        .for_update()
        .load(&conn)
        //~^ ERROR: E0277
        .unwrap();
}
