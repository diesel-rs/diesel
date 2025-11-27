extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let pred = id.eq("string");
    //~^ ERROR: the trait bound `&str: AsExpression<diesel::sql_types::Integer>` is not satisfied
    let pred = id.eq(name);
    //~^ ERROR: the trait bound `columns::name: AsExpression<diesel::sql_types::Integer>` is not satisfied
}
