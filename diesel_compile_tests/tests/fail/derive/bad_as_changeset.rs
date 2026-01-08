use diesel::prelude::*;

table! {
    users(id) {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

#[derive(AsChangeset)]
//~^ ERROR: the trait bound `i32: AppearsOnTable<users::table>` is not satisfied
//~| ERROR: the trait bound `&i32: AsExpression<Nullable<Text>>` is not satisfied
//~| ERROR: the trait bound `i32: AppearsOnTable<users::table>` is not satisfied
//~| ERROR: the trait bound `i32: AppearsOnTable<users::table>` is not satisfied
//~| ERROR: the trait bound `i32: AppearsOnTable<users::table>` is not satisfied
struct User {
    id: String,
    name: i32,
    hair_color: Option<i32>,
}

fn main() {}
