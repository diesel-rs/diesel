#[macro_use]
extern crate diesel;

use diesel::backend::Backend;
use diesel::sql_types::{Integer, VarChar};
use diesel::sqlite::SqliteConnection;
use diesel::deserialize::Queryable;
use diesel::*;

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
#[table_name = "users"]
pub struct NewUser(#[column_name = "name"] String);

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .get_result::<User>(&connection);
    //~^ ERROR: SupportsReturningClause

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(users::name)
        .get_result::<String>(&connection);
    //~^ ERROR: SupportsReturningClause
}
