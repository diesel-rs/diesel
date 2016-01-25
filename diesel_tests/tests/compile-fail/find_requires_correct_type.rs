#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::connection::PgConnection;

table! {
    int_primary_key {
        id -> Serial,
    }
}

table! {
    string_primary_key {
        id -> VarChar,
    }
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    int_primary_key::table.find("1").first(&connection).unwrap();
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
    string_primary_key::table.find(1).first(&connection).unwrap();
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
}
