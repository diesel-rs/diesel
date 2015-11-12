#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
    }
}

table! {
    posts {
        id -> Serial,
    }
}

fn main() {
    let source = users::table.order(posts::id);
    //~^ ERROR type mismatch
}
