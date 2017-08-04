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
    use self::users::dsl::*;

    let command = update(users).set(posts::title.eq("Hello"));
    //~^ ERROR type mismatch
    let command = update(users).set(name.eq(posts::title));
    //~^ ERROR E0277
    //~| ERROR E0271
}
