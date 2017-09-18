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

    insert(&posts::id.eq(1))
        .into(users::table)
        .execute(&conn)
        //~^ ERROR E0599
        .unwrap();
}
