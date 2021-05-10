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
        .skip_locked()
        .load(&mut conn)
        .unwrap();
}
