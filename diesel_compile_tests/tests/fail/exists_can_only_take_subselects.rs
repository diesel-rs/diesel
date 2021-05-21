extern crate diesel;

use diesel::*;
use diesel::dsl::exists;

table! {
    users {
        id -> Integer,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

fn main() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    // Sanity check, no error
    users::table.filter(exists(posts::table.select(posts::id))).execute(&mut conn).unwrap();

    users::table.filter(exists(true));
    users::table.filter(exists(users::id));
}
