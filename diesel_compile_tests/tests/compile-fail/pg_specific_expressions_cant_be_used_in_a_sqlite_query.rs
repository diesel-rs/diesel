#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

use diesel::*;
use diesel::sqlite::SqliteConnection;
use diesel::types::*;
use diesel::expression::dsl::*;
use diesel::pg::upsert::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

#[derive(Insertable)]
#[table_name="users"]
struct NewUser(#[column_name(name)] &'static str);

// NOTE: This test is meant to be comprehensive, but not exhaustive.
fn main() {
    use self::users::dsl::*;
    let connection = SqliteConnection::establish(":memory:").unwrap();

    users.select(id).filter(name.eq(any(Vec::<String>::new())))
        .load::<i32>(&connection);
    //~^ ERROR type mismatch resolving `<diesel::sqlite::SqliteConnection as diesel::Connection>::Backend == diesel::pg::Pg`
    users.select(id).filter(name.is_not_distinct_from("Sean"))
        .load::<i32>(&connection);
    //~^ ERROR type mismatch resolving `<diesel::sqlite::SqliteConnection as diesel::Connection>::Backend == diesel::pg::Pg`
    users.select(id).filter(now.eq(now.at_time_zone("UTC")))
        .load::<i32>(&connection);
    //~^ ERROR type mismatch resolving `<diesel::sqlite::SqliteConnection as diesel::Connection>::Backend == diesel::pg::Pg`
    insert(&NewUser("Sean").on_conflict_do_nothing()).into(users)
        .execute(&connection);
    //~^ ERROR type mismatch resolving `<diesel::sqlite::SqliteConnection as diesel::Connection>::Backend == diesel::pg::Pg`
}
