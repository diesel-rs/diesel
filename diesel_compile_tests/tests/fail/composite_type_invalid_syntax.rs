#[macro_use]
extern crate diesel;

composite_type! {
    12
    //~^ ERROR: macro expansion ignores `12` and any tokens following
}
//~^^^^ ERROR: invalid `composite_type!` syntax

composite_type! {
     some wrong syntax
     //~^ ERROR: expected one of `!` or `::`, found `wrong`
}

fn main() {}
