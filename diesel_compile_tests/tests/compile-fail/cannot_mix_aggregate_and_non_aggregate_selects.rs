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

    let source = users.select(hlist!(id, count(users.star())));
    //~^ ERROR E0277
}
