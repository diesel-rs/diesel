#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::backend::Backend;
use diesel::sqlite::{Sqlite, SqliteQueryBuilder, SqliteConnection};
use diesel::types::{Integer, VarChar};

table! {
    users {
        id -> Integer,
        name -> VarChar,
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

    fn values_bind_params(&self, out: &mut <Sqlite as Backend>::BindCollector) -> QueryResult<()> {
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

    let new_users = vec![
        NewUser("Hello".into()),
        NewUser("World".into()),
    ];
    insert(&new_users)
        .into(users::table)
        .execute(&connection);
    //~^ ERROR: SupportsDefaultKeyword
}
