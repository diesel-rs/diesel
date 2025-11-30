extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = MysqlConnection::establish("").unwrap();
    users::table
        .offset(42)
        .get_result::<(i32, String)>(&mut connection);
    //~^ ERROR: `LimitOffsetClause<NoLimitClause, OffsetClause<Bound<BigInt, i64>>>` is no valid SQL fragment for the `Mysql` backend

    users::table
        .offset(42)
        .into_boxed()
        //~^ ERROR: the trait bound `LimitOffsetClause<NoLimitClause, ...>: IntoBoxedClause<'_, ...>` is not satisfied
        //~| ERROR: the trait bound `LimitOffsetClause<NoLimitClause, ...>: IntoBoxedClause<'_, ...>` is not satisfied
        .get_result::<(i32, String)>(&mut connection);
}
