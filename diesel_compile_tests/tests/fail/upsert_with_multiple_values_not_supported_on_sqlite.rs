extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();

    diesel::insert_into(users::table)
        .values(vec![users::id.eq(42), users::id.eq(43)])
        .on_conflict_do_nothing()
        .execute(&mut connection);
    //~^ ERROR: type mismatch resolving `<Sqlite as SqlDialect>::InsertWithDefaultKeyword == IsoSqlDefaultKeyword`
    //~| ERROR: `BatchInsert<Vec<ValuesClause<..., ...>>, ..., (), false>` is no valid SQL fragment for the `Sqlite` backend
}
