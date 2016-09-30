#![feature(custom_derive, custom_attribute, plugin)]
#![plugin(diesel_codegen)]

#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)]
#[table_name="users"]
struct User {
    id: i32,
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    insert_or_replace(&User { id: 1 }).into(users::table).execute(&connection).unwrap();
    //~^ ERROR type mismatch resolving `<diesel::pg::PgConnection as diesel::Connection>::Backend == diesel::sqlite::Sqlite`
}
