#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::{max, min};

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    let source = stuff::table.select(max(stuff::b));
    //~^ ERROR SqlOrd
    //~| ERROR E0277
    let source = stuff::table.select(min(stuff::b));
    //~^ ERROR SqlOrd
    //~| ERROR E0277
}
