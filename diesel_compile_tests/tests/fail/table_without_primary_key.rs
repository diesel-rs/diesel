#[macro_use] extern crate diesel;

table! {
    user {
        user_id -> Integer,
        name -> Text,
    }
}

fn main() {}
