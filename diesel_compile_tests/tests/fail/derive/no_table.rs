#[macro_use]
extern crate diesel;

#[derive(AsChangeset)]
struct User {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserForm {
    id: i32,
    name: String,
}

fn main() {}
