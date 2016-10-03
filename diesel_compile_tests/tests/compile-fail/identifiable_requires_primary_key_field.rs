#![feature(custom_derive, plugin, custom_attribute, rustc_macro)]
#![plugin(diesel_codegen_old)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

#[derive(Identifiable)] //~ ERROR Could not find a field named `id` on `User`
//~^ WARNING
pub struct User {
    name: String,
    hair_color: Option<String>,
}

fn main() {}
