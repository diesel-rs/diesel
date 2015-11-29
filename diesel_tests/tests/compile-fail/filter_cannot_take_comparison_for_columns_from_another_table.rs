#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Serial,
        title -> VarChar,
    }
}

fn main() {
    let _ = users::table.filter(posts::id.eq(1));
    //~^ ERROR SelectableExpression
    let _ = users::table.filter(users::name.eq(posts::title));
    //~^ ERROR SelectableExpression
}
