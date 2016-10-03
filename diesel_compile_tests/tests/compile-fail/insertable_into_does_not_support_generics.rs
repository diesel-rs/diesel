#![feature(custom_derive, plugin, custom_attribute, rustc_macro)]
#![plugin(diesel_codegen_old)]

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

#[derive(Insertable)] //~ WARNING
//~^ ERROR #[derive(Insertable)] does not support generic types
#[table_name="users"]
pub struct NewUser<T> {
    name: T,
}
