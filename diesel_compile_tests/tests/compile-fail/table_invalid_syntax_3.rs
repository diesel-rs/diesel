#[macro_use] extern crate diesel;

table! {
     #[foobar]
     posts {
         id -> Integer,
     }
}
// error-pattern: invalid table! syntax

fn main() {}
