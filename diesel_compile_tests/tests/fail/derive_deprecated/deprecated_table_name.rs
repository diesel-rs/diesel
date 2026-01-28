#[macro_use]
extern crate diesel;

#[derive(AsChangeset)]
#[table_name = "users"]
//~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `users`
struct UserForm1 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name]
//~^ ERROR: unexpected end of input, expected `=`
struct UserForm2 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name()]
//~^ ERROR: expected `=`
struct UserForm3 {
    name: String,
}

#[derive(AsChangeset)]
#[table_name = 1]
//~^ ERROR: expected string literal
struct UserForm4 {
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[table_name = "1"]
//~^ ERROR: expected identifier
struct UserForm5 {
    id: i32,
    name: String,
}

fn main() {}
