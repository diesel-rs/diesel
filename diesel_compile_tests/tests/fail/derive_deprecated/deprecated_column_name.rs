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
    #[column_name = "name"]
    //~^ ERROR: cannot find type `name` in module `users`
    //~| ERROR: cannot find value `name` in module `users`
    full_name: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserTuple(#[column_name = "name"] String);
//~^ ERROR: cannot find type `name` in module `users`
//~| ERROR: cannot find value `name` in module `users`

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct2 {
    #[column_name]
    //~^ ERROR: unexpected end of input, expected `=`
    id: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct3 {
    #[column_name()]
    //~^ ERROR: expected `=`
    id: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct4 {
    #[column_name = 1]
    //~^ ERROR: expected string literal
    id: i32,
}

#[derive(AsChangeset)]
#[diesel(table_name = users)]
struct UserStruct5 {
    #[column_name = "1"]
    //~^ ERROR: expected string literal
    id: i32,
}

fn main() {}
