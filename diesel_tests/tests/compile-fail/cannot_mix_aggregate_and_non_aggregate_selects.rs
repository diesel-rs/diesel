#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::count;
use diesel::connection::PgConnection;

table! {
    users {
        id -> Serial,
    }
}

fn main() {
    use self::users::dsl::*;

    let connection = PgConnection::establish("").unwrap();
    let source = users.select((id, count(users.star())));
    //~^ ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
}
