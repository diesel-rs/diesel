#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();

    diesel::insert_into(users::table)
        .values(vec![users::id.eq(42), users::id.eq(43)])
        .on_conflict_do_nothing()
        .execute(&connection);
}
