#![feature(custom_derive, plugin, custom_attribute)]
#![plugin(diesel_codegen)]
#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
        hair_color -> VarChar,
    }
}

#[derive(Queryable)]
#[changeset_for(users)]
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
