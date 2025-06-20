#[macro_use]
extern crate diesel;

table! {
    12
    //~^ ERROR: macro expansion ignores `12` and any tokens following
}
//~^^^^ ERROR: invalid `table!` syntax

fn main() {}
