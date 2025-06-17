#[macro_use]
extern crate diesel;

#[derive(Associations)]
//~^ ERROR: At least one `belongs_to` is needed for deriving `Associations` on a structure.
struct User {
    id: i32,
}

fn main() {}
