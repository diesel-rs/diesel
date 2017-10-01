#[macro_use] extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
//~^ ERROR proc-macro derive panicked
//~| HELP `#[derive(Insertable)]` does not support generic types
#[table_name="users"]
pub struct NewUser<T> {
    name: T,
}
