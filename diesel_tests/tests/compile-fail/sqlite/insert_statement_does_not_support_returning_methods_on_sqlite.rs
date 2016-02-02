#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::*;
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

use diesel::persistable::InsertValues;
use diesel::query_builder::BuildQueryResult;

// It doesn't actually matter if this would work. We're testing that insert fails
// to compile here.
pub struct MyValues;
impl InsertValues<Sqlite> for MyValues {
    fn column_names(&self, out: &mut SqliteQueryBuilder) -> BuildQueryResult {
        Ok(())
    }

    fn values_clause(&self, out: &mut SqliteQueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

impl<'a> Insertable<users::table, Sqlite> for &'a NewUser {
    type Values = MyValues;

    fn values(self) -> Self::Values {
        MyValues
    }
}

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    insert(&NewUser("Hello".into()))
        .into(users::table)
        .get_result::<User>(&connection);
    //~^ ERROR: SupportsReturningClause
}
