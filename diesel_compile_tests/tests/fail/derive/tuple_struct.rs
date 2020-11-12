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
struct User(i32, #[diesel(column_name = name)] String, String);

fn main() {}
