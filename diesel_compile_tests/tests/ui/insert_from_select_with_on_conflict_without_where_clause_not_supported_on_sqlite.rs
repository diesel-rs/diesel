extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();

    users::table.select(users::id)
        .insert_into(users::table)
        .into_columns(users::id)
        .on_conflict(users::id)
        .do_nothing()
        .execute(&connection)
        .unwrap();

}
