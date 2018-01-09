#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;
use diesel::backend::Backend;
use diesel::sql_types::{Integer, VarChar};

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

use diesel::deserialize::FromSqlRow;

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
