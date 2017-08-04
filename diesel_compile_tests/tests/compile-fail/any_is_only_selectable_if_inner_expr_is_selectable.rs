#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::dsl::*;

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

fn main() {
    use self::stuff::dsl::*;

    stuff.filter(name.eq(any(more_stuff::names)));
    //~^ ERROR E0277
    //~| ERROR E0271
}
