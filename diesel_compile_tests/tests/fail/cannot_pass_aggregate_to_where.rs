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
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::Yes: MixedAggregates<diesel::expression::is_aggregate::No>` is not satisfied
}
