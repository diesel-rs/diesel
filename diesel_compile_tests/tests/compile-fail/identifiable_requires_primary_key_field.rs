#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen_old)]

#[macro_use]
extern crate diesel;

#[derive(Identifiable)] //~ ERROR Could not find a field named `id` on `User`
//~^ WARNING
pub struct User {
    name: String,
    hair_color: Option<String>,
}

fn main() {}
