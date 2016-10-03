#![feature(rustc_macro)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
        hair_color -> VarChar,
    }
}

#[derive(Queryable, AsChangeset)]
#[table_name = "users"]
pub struct User {
    name: String,
    hair_color: String,
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    let mut user = User {
        name: "Sean".to_string(),
        hair_color: "black".to_string(),
    };
    user.save_changes(&connection);
    //~^ ERROR no method named `save_changes` found
}
