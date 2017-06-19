#[macro_use] extern crate diesel;

table! {
     some wrong syntax
}
// error-pattern: invalid table! syntax

fn main() {}
