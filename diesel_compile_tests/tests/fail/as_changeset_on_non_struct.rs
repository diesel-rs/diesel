#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

#[derive(AsChangeset)]
#[table_name = "users"]
enum User {}

fn main() {}
