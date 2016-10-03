#![feature(custom_derive, plugin, custom_attribute, rustc_macro)]
#![plugin(diesel_codegen_old)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)] //~ WARNING
#[table_name="users"]
struct User {
    id: i32,
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    insert_or_replace(&User { id: 1 }).into(users::table).execute(&connection).unwrap();
    //~^ ERROR type mismatch resolving `<diesel::pg::PgConnection as diesel::Connection>::Backend == diesel::sqlite::Sqlite`
}
