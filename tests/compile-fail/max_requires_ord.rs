#[macro_use]
extern crate yaqb;

use yaqb::*;
use yaqb::expression::max;

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    let source = stuff::table.select(max(stuff::b));
    //~^ ERROR E0277
}
