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
    name: i32,
}

fn main() {}
