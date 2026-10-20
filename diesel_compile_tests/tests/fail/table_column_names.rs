#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        users -> Integer,
        //~^ ERROR: column `users` cannot be named the same as its table.
    }
}

fn main() {}
