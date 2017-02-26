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
    let _ = users::table.filter(posts::id.eq(1));
    //~^ ERROR AppearsOnTable
    let _ = users::table.filter(users::name.eq(posts::title));
    //~^ ERROR AppearsOnTable
}
