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
    let mut conn = PgConnection::establish("").unwrap();

    insert_into(users::table)
        .values(&posts::id.eq(1));

    insert_into(users::table)
        .values(&(posts::id.eq(1), users::id.eq(2)));
        //FIXME: Bad error on the second one
}
