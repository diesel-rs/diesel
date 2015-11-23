#[macro_use]
extern crate yaqb;

use yaqb::*;
use yaqb::expression::count;

table! {
    users {
        id -> Serial,
    }
}

fn main() {
    use self::users::dsl::*;

    let source = users.filter(count(id).gt(3));
    //~^ ERROR NonAggregate
}
