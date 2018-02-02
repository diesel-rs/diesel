#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[table_name(users)]
struct UserForm {
    id: i32,
    #[column_name(name)]
    name: String,
}

fn main() {}
