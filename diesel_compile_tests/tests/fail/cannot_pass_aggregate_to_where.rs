extern crate diesel;

use diesel::dsl::count;
use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    use self::users::dsl::*;

    let source = users.filter(count(id).gt(3));
    //~^ ERROR:  mixing aggregate and not aggregate expressions is not allowed in SQL
}
