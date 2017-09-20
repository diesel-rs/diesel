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
        .values(&posts::id.eq(1))
        .execute(&conn)
        //~^ ERROR E0599
        .unwrap();

    insert_into(users::table)
        .values(&(posts::id.eq(1), users::id.eq(2)))
        .execute(&conn)
        //~^ ERROR E0599
        .unwrap();
}
