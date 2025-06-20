#[macro_use]
extern crate diesel;

table! {
     some wrong syntax
     //~^ ERROR: expected one of `!` or `::`, found `wrong`
}

fn main() {}
