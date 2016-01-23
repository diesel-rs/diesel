#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::backend::Backend;
use diesel::connection::SqliteConnection;
use diesel::types::{Integer, VarChar};

table! {
    users {
        id -> Serial,
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

use diesel::expression::AsExpression;
use diesel::expression::grouped::Grouped;
use diesel::expression::helper_types::AsExpr;

impl<'a> Insertable<users::table> for &'a NewUser {
    type Columns = users::name;
    type Values = Grouped<AsExpr<&'a String, users::name>>;

    fn columns() -> Self::Columns {
        users::name
    }

    fn values(self) -> Self::Values {
        Grouped(<&'a String as AsExpression<VarChar>>::as_expression(&self.0))
    }
}

fn main() {
    let connection = SqliteConnection::establish(":memory:").unwrap();

    insert(&NewUser("Hello".into()))
        .into(users::table)
        .get_result::<User>(&connection);
    //~^ ERROR: SupportsReturningClause
}
