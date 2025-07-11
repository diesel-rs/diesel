use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Insertable)]
//~^ ERROR: the trait bound `std::string::String: diesel::Expression` is not satisfied
//~| ERROR: the trait bound `i32: diesel::Expression` is not satisfied
//~| ERROR: the trait bound `std::string::String: diesel::Expression` is not satisfied
//~| ERROR: the trait bound `i32: diesel::Expression` is not satisfied
struct User {
    id: String,
    //~^ ERROR: the trait bound `std::string::String: diesel::Expression` is not satisfied
    //~| ERROR: the trait bound `std::string::String: diesel::Expression` is not satisfied
    name: i32,
    //~^ ERROR: the trait bound `i32: diesel::Expression` is not satisfied
    //~| ERROR: the trait bound `i32: diesel::Expression` is not satisfied
}

fn main() {}
