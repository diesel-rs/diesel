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
#[diesel(table_name = users)]
enum UserEnum {}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct;

fn main() {}
