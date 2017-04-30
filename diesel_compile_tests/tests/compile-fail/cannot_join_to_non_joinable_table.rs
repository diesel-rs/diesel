#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

fn main() {
    let _ = users::table.inner_join(posts::table);
    //~^ ERROR E0277
    //~| ERROR E0277
    let _ = users::table.left_outer_join(posts::table);
    //~^ ERROR E0277
    //~| ERROR E0277
}
