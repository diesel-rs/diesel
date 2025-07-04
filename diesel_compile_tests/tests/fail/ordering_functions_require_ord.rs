extern crate diesel;

use diesel::dsl::{max, min};
use diesel::*;

table! {
    stuff (b) {
        b -> Bool,
    }
}

fn main() {
    let source = stuff::table.select(max(stuff::b));
    //~^ ERROR: the trait bound `Bool: SqlOrdAggregate` is not satisfied
    //~| ERROR: expressions of the type `diesel::sql_types::Bool` cannot be ordered by the database
    let source = stuff::table.select(min(stuff::b));
    //~^ ERROR: the trait bound `Bool: SqlOrdAggregate` is not satisfied
    //~| ERROR: expressions of the type `diesel::sql_types::Bool` cannot be ordered by the database
}
