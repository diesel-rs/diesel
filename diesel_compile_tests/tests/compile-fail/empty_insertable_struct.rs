#[macro_use] extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)]
//~^ ERROR proc-macro derive panicked
//~| HELP Failed to derive `Insertable` for `NewUser`: `Insertable` cannot be used on structs with empty fields
#[table_name="users"]
pub struct NewUser {}

fn main() {
}
