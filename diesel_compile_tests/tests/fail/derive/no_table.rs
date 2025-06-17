#[macro_use]
extern crate diesel;

#[derive(AsChangeset)]
struct User {
    //~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `users`
    id: i32,
    name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
//~^ ERROR: failed to resolve: use of unresolved module or unlinked crate `users`
struct UserForm {
    id: i32,
    name: String,
}

fn main() {}
