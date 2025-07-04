extern crate diesel;

use diesel::sqlite::SqliteConnection;
use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut conn = SqliteConnection::establish("").unwrap();
    users::table
        .for_update()
        .load(&mut conn)
        //~^ ERROR: `diesel::query_builder::locking_clause::ForUpdate` is no valid SQL fragment for the `Sqlite` backend
        //~| ERROR: `diesel::query_builder::locking_clause::NoModifier` is no valid SQL fragment for the `Sqlite` backend
        .unwrap();
}
