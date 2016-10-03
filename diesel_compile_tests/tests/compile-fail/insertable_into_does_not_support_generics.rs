#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen_old)]

#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)] //~ WARNING
//~^ ERROR #[derive(Insertable)] does not support generic types
#[table_name="users"]
pub struct NewUser<T> {
    name: T,
}
