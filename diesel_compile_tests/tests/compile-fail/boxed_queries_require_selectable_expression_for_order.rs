#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::Pg;

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
    users::table.into_boxed::<Pg>().order(posts::title.desc());
    //~^ ERROR AppearsOnTable
}
