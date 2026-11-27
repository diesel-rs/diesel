extern crate diesel;

use diesel::sqlite::{SqliteBindValue, SqliteTableFunctionRow};

struct LocalSqlType;

impl SqliteTableFunctionRow<LocalSqlType> for (i32,) {
    //~^ ERROR: the trait bound `(i32,): ResultRow<LocalSqlType>` is not satisfied
    type Values = [SqliteBindValue<'static>; 1];

    fn values(&self) -> diesel::QueryResult<Self::Values> {
        //~^ ERROR: the trait bound `(i32,): ResultRow<LocalSqlType>` is not satisfied
        Ok([SqliteBindValue::from(self.0)])
    }
}

fn main() {}
