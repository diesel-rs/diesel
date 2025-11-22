use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
struct User {
    id: String,
    //~^ ERROR: the trait bound `std::string::String: AsExpression<diesel::sql_types::Integer>` is not satisfied
    name: i32,
    //~^ ERROR: the trait bound `i32: AsExpression<diesel::sql_types::Text>` is not satisfied
}

fn main() {}
