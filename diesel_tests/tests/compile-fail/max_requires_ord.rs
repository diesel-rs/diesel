#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::max;

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    let source = stuff::table.select(max(stuff::b));
    //~^ ERROR E0277
}
