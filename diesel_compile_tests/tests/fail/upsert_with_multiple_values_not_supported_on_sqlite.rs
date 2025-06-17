extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser {
    id: i32,
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();

    // it works with plain values
    diesel::insert_into(users::table)
        .values(vec![users::id.eq(42), users::id.eq(43)])
        .on_conflict_do_nothing()
        .execute(&mut connection)
        .unwrap();

    // it's not supported for insertable structs which might require default value
    // handling

    diesel::insert_into(users::table)
        .values(Vec::<NewUser>::new())
        .on_conflict_do_nothing()
        .execute(&mut connection)
        //~^ ERROR: type mismatch resolving `<Sqlite as SqlDialect>::InsertWithDefaultKeyword == IsoSqlDefaultKeyword`
        //~| ERROR: `BatchInsert<Vec<ValuesClause<(...,), ...>>, ..., (), false>` is no valid SQL fragment for the `Sqlite` backend
        .unwrap();
}
