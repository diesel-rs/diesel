#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::min;

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    let source = stuff::table.select(min(stuff::b));
    //~^ ERROR E0277
}
