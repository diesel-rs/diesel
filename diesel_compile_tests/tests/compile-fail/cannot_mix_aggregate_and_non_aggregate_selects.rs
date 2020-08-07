#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::count_star;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    let source = users.select((id, count_star()));
    //~^ ERROR MixedAggregates
}
