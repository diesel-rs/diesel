#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;
use diesel::backend::Backend;
use diesel::types::{Integer, VarChar};

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

pub struct User {
    id: i32,
    name: String,
}

use diesel::types::FromSqlRow;

impl<DB: Backend> Queryable<(Integer, VarChar), DB> for User where
    (i32, String): FromSqlRow<(Integer, VarChar), DB>,
{
    type Row = (i32, String);

    fn build(row: Self::Row) -> Self {
        User {
            id: row.0,
            name: row.1,
        }
    }
}

pub struct NewUser(String);

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    insert_default_values()
        .into(users::table)
        .get_result::<User>(&connection);
    //~^ ERROR: no method named `get_result`

    insert_default_values()
        .into(users::table)
        .returning(users::name)
        .get_result::<String>(&connection);
    //~^ ERROR: SupportsReturningClause
}
