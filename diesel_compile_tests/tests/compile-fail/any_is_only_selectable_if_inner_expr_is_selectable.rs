#[macro_use] extern crate diesel;

use diesel::*;
use diesel::dsl::*;

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

    let conn = PgConnection::establish("").unwrap();

    let _ = LoadDsl::load::<Stuff>(
    //~^ ERROR E0277
        stuff.filter(name.eq(any(more_stuff::names))),
        &conn,
    );
}
