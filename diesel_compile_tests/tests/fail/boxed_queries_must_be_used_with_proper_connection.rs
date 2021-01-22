extern crate diesel;

use diesel::pg::Pg;
use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    users::table.into_boxed::<Pg>().load::<(i32,)>(&connection);
}
