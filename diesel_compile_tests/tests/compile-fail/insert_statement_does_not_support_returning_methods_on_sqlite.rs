#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

use diesel::*;
use diesel::sqlite::{Sqlite, SqliteQueryBuilder, SqliteConnection};
use diesel::backend::Backend;
use diesel::types::{Integer, VarChar};

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Queryable)]
pub struct User {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser(#[column_name(name)] String);

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    insert(&NewUser("Hello".into()))
        .into(users::table)
        .get_result::<User>(&connection);
    //~^ ERROR: SupportsReturningClause

    insert(&NewUser("Hello".into()))
        .into(users::table)
        .returning(users::name)
        .get_result::<String>(&connection);
    //~^ ERROR: SupportsReturningClause
}
