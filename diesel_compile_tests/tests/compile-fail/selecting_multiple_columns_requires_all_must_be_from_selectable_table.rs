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
        user_id -> Integer,
    }
}

fn main() {
    let stuff = users::table.select((posts::id, posts::user_id));
    //~^ ERROR Selectable
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    let stuff = users::table.select((posts::id, users::name));
    //~^ ERROR Selectable
    //~| ERROR E0277
}
