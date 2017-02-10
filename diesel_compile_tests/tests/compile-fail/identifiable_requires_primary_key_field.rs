#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

#[derive(Identifiable)]
//~^ ERROR proc-macro derive panicked
//~| HELP Could not find a field named `id` on `User`
pub struct User {
    name: String,
    hair_color: Option<String>,
}

fn main() {}
