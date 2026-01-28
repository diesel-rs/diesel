extern crate diesel;

use diesel::pg::Pg;
use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();
    users::table
        .into_boxed::<Pg>()
        .load::<(i32,)>(&mut connection);
    //~^ ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`
}
