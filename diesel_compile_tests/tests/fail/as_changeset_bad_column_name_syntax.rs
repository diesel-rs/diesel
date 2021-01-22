#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[table_name = "users"]
struct User {
    #[column_name]
    name: String,
}

fn main() {}
