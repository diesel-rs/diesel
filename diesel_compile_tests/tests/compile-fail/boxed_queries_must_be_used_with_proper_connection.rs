#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::Pg;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    users::table.into_boxed::<Pg>().load::<(i32,)>(&connection);
    //~^ ERROR type mismatch
}
