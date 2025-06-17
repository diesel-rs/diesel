#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct1 {
    name: String,
    //~^ ERROR: cannot find type `name` in module `users`
    //~| ERROR: cannot find value `name` in module `users`
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct2 {
    #[diesel(column_name = name)]
    //~^ ERROR: cannot find type `name` in module `users`
    //~| ERROR: cannot find value `name` in module `users`
    full_name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserTuple(#[diesel(column_name = name)] String);
//~^ ERROR: cannot find type `name` in module `users`
//~| ERROR: cannot find value `name` in module `users`

fn main() {}
