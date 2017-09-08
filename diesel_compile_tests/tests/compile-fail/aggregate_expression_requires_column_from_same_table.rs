#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

fn main() {
    use diesel::dsl::*;
    let source = users::table.select(sum(posts::id));
    //~^ ERROR E0277
    //~| ERROR AppearsInFromClause
    //~| ERROR E0277
    let source = users::table.select(avg(posts::id));
    //~^ ERROR E0277
    //~| ERROR AppearsInFromClause
    //~| ERROR E0277
    let source = users::table.select(max(posts::id));
    //~^ ERROR E0277
    //~| ERROR AppearsInFromClause
    //~| ERROR E0277
    let source = users::table.select(min(posts::id));
    //~^ ERROR E0277
    //~| ERROR AppearsInFromClause
    //~| ERROR E0277
}
