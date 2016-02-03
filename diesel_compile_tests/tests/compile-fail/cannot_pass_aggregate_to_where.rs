#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::count;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    let source = users.filter(count(id).gt(3));
    //~^ ERROR NonAggregate
}
