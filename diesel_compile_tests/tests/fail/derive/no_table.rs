#[macro_use]
extern crate diesel;

#[derive(AsChangeset)]
struct User {
    //~^ ERROR: cannot find module or crate `users` in this scope
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
//~^ ERROR: cannot find module or crate `users` in this scope
struct UserForm {
    id: i32,
    name: String,
}

fn main() {}
