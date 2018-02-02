#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[primary_key(id, bar = "baz", qux(id))]
struct UserForm {
    id: i32,
    name: String,
}

fn main() {}
