extern crate diesel;

use diesel::pg::PgConnection;
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
    let mut conn = PgConnection::establish("").unwrap();

    insert_into(users::table).values(&posts::id.eq(1));
    //~^ ERROR: type mismatch resolving `<id as Column>::Table == table`

    insert_into(users::table).values(&(posts::id.eq(1), users::id.eq(2)));
    //~^ ERROR: type mismatch resolving `<&... as Insertable<...>>::Values == ValuesClause<..., ...>`
    //~| ERROR: type mismatch resolving `<id as Column>::Table == table`
    //~| ERROR: type mismatch resolving `<&... as Insertable<...>>::Values == ValuesClause<..., ...>`
    //FIXME: Bad error on the second one
}
