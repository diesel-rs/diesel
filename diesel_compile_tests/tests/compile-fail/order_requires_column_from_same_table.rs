#[macro_use]
extern crate diesel;

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
    let source = users::table.order(posts::id);
    //~^ ERROR E0277
    //~| ERROR E0271
}
