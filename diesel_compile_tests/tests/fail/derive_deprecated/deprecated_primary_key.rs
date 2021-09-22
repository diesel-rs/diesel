#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(AsChangeset)]
#[primary_key(id, bar = "baz")]
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key(id, qux(id))]
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key]
struct UserForm3 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[primary_key = id]
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
#[primary_key(id, name)]
struct UserForm5 {
    id: i32,
    name: String,
}

fn main() {}
