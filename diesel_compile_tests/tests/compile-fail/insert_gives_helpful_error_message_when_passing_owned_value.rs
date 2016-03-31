#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen)]

#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[insertable_into(users)]
struct NewUser<'a> {
    name: &'a str,
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    let new_user = NewUser { name: "Sean" };
    insert(new_user).into(users::table).execute(&connection).unwrap();
    //~^ ERROR mismatched types
    //~| expected `&_`
    //~| found `NewUser<'_>`
    //~| ERROR E0275
    // The E0275 is a quirk in Rust, not something we actually care about
}
