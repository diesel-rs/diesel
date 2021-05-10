extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    update(users::table)
        .execute(&mut conn);
}
