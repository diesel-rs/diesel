#[macro_use] extern crate diesel;

table! {
     some wrong syntax
}
//~^ ERROR environment variable `invalid table! syntax` not defined

fn main() {}
