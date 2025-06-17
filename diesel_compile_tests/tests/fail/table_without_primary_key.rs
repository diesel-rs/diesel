#[macro_use]
extern crate diesel;

table! {
    user {
        //~^ ERROR: Neither an explicit primary key found nor does an `id` column exist.
        user_id -> Integer,
        name -> Text,
    }
}

fn main() {}
