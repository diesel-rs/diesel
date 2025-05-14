extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let conn = &mut PgConnection::establish("…").unwrap();
    users::table
        .filter(users::id.eq(1).and(users::id).or(users::id))
        .select(users::id)
        .execute(conn)
        .unwrap();
}
