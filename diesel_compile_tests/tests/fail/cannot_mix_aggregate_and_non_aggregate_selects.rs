extern crate diesel;

use diesel::dsl::count_star;
use diesel::sql_types::{Integer, Nullable};
use diesel::*;

table! {
    users {
        id -> Integer,
        nullable_int_col -> Nullable<Integer>,
    }
}

define_sql_function!(fn f(x: Nullable<Integer>, y: Nullable<Integer>) -> Nullable<Integer>);

fn main() {
    use self::users::dsl::*;
    use diesel::dsl::max;

    let source = users.select((id, count_star()));
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied

    let source = users.select(nullable_int_col + max(nullable_int_col));
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied

    let source = users.select(f(nullable_int_col, max(nullable_int_col)));
    //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied
}
