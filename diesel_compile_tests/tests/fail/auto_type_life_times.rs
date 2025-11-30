use diesel::dsl::*;
use diesel::prelude::*;

diesel::table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[auto_type]
fn with_lifetime(name: &'_ str) -> _ {
    //~^ ERROR: `#[auto_type]` requires named lifetimes
    //~| ERROR: missing lifetime specifier
    users::table.filter(users::name.eq(name))
}

#[auto_type]
fn with_lifetime2(name: &str) -> _ {
    //~^ ERROR: `#[auto_type]` requires named lifetimes
    //~| ERROR: missing lifetime specifier
    users::table.filter(users::name.eq(name))
}

fn main() {
    println!("Hello, world!");
}
