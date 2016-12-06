#[macro_use]
extern crate diesel;

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

impl_Insertable! {
    (users)
    pub struct NewUser(#[column_name(name)] String,);
}

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
