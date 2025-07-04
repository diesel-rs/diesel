extern crate diesel;

use diesel::dsl::exists;
use diesel::*;

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
    users::table
        .filter(exists(posts::table.select(posts::id)))
        .execute(&mut conn)
        .unwrap();

    users::table.filter(exists(true));
    //~^ ERROR: the trait bound `bool: SelectQuery` is not satisfied
    users::table.filter(exists(users::id));
    //~^ ERROR: the trait bound `users::columns::id: SelectQuery` is not satisfied
}
