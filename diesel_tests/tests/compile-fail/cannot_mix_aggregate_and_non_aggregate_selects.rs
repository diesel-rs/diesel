#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::expression::count;

table! {
    users {
        id -> Serial,
    }
}

fn main() {
    use self::users::dsl::*;

    let connection = Connection::establish("").unwrap();
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
