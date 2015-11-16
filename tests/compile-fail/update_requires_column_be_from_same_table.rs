#[macro_use]
extern crate yaqb;

use yaqb::*;
use yaqb::query_builder::update;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Serial,
        title -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let command = update(users).set(posts::title.eq("Hello"));
    //~^ ERROR type mismatch
    let command = update(users).set(name.eq(posts::title));
    //~^ ERROR type mismatch
}
