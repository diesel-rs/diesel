#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use diesel::dsl::sum;

    let _ = users::table.filter(users::name);
    //~^ ERROR the trait bound `diesel::sql_types::Text: diesel::sql_types::BoolOrNullableBool` is not satisfied
    let _ = users::table.filter(sum(users::id).eq(1));
    //~^ ERROR MixedAggregates
}
