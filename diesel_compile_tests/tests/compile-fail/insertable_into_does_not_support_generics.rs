#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
//~^ ERROR custom derive attribute panicked
//~| HELP `#[derive(Insertable)]` does not support generic types
#[table_name="users"]
pub struct NewUser<T> {
    name: T,
}
