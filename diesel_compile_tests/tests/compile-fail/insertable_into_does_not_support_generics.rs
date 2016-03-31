#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen)]

#[macro_use]
extern crate diesel;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[insertable_into(users)]
pub struct NewUser<T> { //~ ERROR #[insertable_into] does not support generic types
    name: T,
}
