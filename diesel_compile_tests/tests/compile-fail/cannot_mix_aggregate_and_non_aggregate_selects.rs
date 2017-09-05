#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::count;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    let source = users.select((id, count(users.star())));
    //~^ ERROR E0277
}
