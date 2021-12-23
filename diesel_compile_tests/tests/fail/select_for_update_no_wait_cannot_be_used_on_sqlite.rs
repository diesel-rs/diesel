extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut conn = SqliteConnection::establish("").unwrap();
    users::table
        .for_update()
        .no_wait()
        .load(&mut conn)
        .unwrap();
}
