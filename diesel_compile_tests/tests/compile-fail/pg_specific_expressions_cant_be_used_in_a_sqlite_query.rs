#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;
use diesel::types::*;
use diesel::expression::dsl::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

// NOTE: This test is meant to be comprehensive, but not exhaustive.
fn main() {
    use self::users::dsl::*;
    let connection = SqliteConnection::establish(":memory:").unwrap();

    users.select(id).filter(name.eq(any(Vec::<String>::new())))
        .load::<i32>(&connection);
    //~^ ERROR type mismatch resolving `<diesel::sqlite::SqliteConnection as diesel::Connection>::Backend == diesel::pg::Pg`
    users.select(id).filter(name.is_not_distinct_from("Sean"))
        .load::<i32>(&connection);
    //~^ ERROR E0277
    let n = lower("sean").aliased("n");
    users.with(n).select(id)
        .load::<i32>(&connection);
    //~^ ERROR E0277
    users.select(id).filter(now.eq(now.at_time_zone("UTC")))
        .load::<i32>(&connection);
    //~^ ERROR E0277
}
