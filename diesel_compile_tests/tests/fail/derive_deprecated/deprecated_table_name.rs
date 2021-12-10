#[macro_use]
extern crate diesel;

#[derive(AsChangeset)]
#[table_name = "users"]
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name]
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name()]
struct UserForm3 {
    name: String,
}

#[derive(AsChangeset)]
#[table_name = 1]
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "1"]
struct UserForm5 {
    id: i32,
    name: String,
}

fn main() {}
