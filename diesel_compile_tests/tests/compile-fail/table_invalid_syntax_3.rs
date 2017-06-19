#[macro_use] extern crate diesel;

table! {
     #[foobar]
     //~^ ERROR expected ident, found #
     posts {
         id -> Integer,
     }
}

fn main() {}
