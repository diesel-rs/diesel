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
    //~^ ERROR type mismatch resolving `<users::columns::name as diesel::Expression>::SqlType == diesel::sql_types::Bool`
    let _ = users::table.filter(sum(users::id).eq(1));
    //~^ ERROR MixedAggregates
}
