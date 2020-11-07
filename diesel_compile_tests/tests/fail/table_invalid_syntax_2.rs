#[macro_use] extern crate diesel;

table! {
     some wrong syntax
}
// error-pattern: Invalid `table!` syntax. Please see the `table!` macro docs for more info.

fn main() {}
