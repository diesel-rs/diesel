#[macro_use]
extern crate diesel;

composite_type! {
    users {
        id -> Integer,
        users -> Integer,
        //~^ ERROR: column `users` cannot be named the same as its composite_type.
    }
}

fn main() {}
