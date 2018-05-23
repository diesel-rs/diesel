#[macro_use] extern crate diesel;

table! {
     #[foobar]
     posts {
         id -> Integer,
     }
}
// error-pattern: E0658

fn main() {}
