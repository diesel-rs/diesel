#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::connection::PgConnection;

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
    let connection = PgConnection::establish("").unwrap();
    let select_id = users::table.select(posts::id);
    //~^ ERROR SelectableExpression
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
}
