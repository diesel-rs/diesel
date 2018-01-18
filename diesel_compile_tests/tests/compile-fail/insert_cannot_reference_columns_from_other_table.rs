#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

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
    let conn = PgConnection::establish("").unwrap();

    insert_into(users::table)
        .values(&posts::id.eq(1));
        //~^ ERROR type mismatch resolving `<posts::columns::id as diesel::Column>::Table == users::table`

    insert_into(users::table)
        .values(&(posts::id.eq(1), users::id.eq(2)));
        //~^ ERROR type mismatch resolving `<posts::columns::id as diesel::Column>::Table == users::table`
        //~| ERROR E0271
        //FIXME: Bad error on the second one
}
