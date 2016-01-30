#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::{sum, avg};

table! {
    time_key {
        id -> Time,
    }
}

table! {
    string_primary_key {
        id -> VarChar,
    }
}

fn main() {
    time_key::table.select(sum(time_key::id));
    //~^ ERROR E0277
    //~| ERROR E0277
    string_primary_key::table.select(avg(string_primary_key::id));
    //~^ ERROR E0277
    //~| ERROR E0277
}
