extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();

    users::table
        .select(users::id)
        .insert_into(users::table)
        .into_columns(users::id)
        .on_conflict(users::id)
        .do_nothing()
        .execute(&mut connection)
        //~^ ERROR: `OnConflictSelectWrapper<SelectStatement<..., ...>>` is no valid SQL fragment for the `Sqlite` backend
        .unwrap();
}
