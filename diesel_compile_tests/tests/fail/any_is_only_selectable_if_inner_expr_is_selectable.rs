extern crate diesel;

use diesel::dsl::*;
use diesel::*;

table! {
    stuff {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    more_stuff (names) {
        names -> Array<VarChar>,
    }
}

#[derive(Queryable)]
struct Stuff {
    id: i32,
    name: String,
}

fn main() {
    use self::stuff::dsl::*;

    let mut conn = PgConnection::establish("").unwrap();

    let _ = stuff.filter(name.eq(any(more_stuff::names)))
        .load(&mut conn);
}
