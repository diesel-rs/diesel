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
        .skip_locked()
        .load(&mut conn)
        //~^ ERROR: `diesel::query_builder::locking_clause::ForUpdate` is no valid SQL fragment for the `Sqlite` backend
        //~| ERROR: `diesel::query_builder::locking_clause::SkipLocked` is no valid SQL fragment for the `Sqlite` backend
        //~| ERROR: the trait bound `{type error}: FromSqlRow<(diesel::sql_types::Integer,), Sqlite>` is not satisfied
        .unwrap();
}
