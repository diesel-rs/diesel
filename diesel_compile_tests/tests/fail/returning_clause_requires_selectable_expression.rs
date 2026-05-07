extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] String);

table! {
    non_users {
        id -> Integer,
        noname -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();

    delete(users::table.filter(users::columns::name.eq("Bill")))
        .returning(non_users::columns::noname);
    //~^ ERROR: `non_users::columns::noname` cannot appear in the `RETURNING` clause of a `DeleteStmt` on `users::table`
    //~| ERROR: type mismatch resolving `<ReturningQuerySource<..., ...> as AppearsInFromClause<...>>::Count == Once`
    //~| ERROR: the trait bound `ReturningQuerySource<..., ...>: TableNotEqual<...>` is not satisfied
    //~| ERROR: the trait bound `ReturningQuerySource<DeleteStmt, users::table>: Table` is not satisfied

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(non_users::columns::noname);
    //~^ ERROR: `non_users::columns::noname` cannot appear in the `RETURNING` clause of a `InsertStmtWithoutOnConflictDoUpdate` on `users::table`
    //~| ERROR: type mismatch resolving `<ReturningQuerySource<..., ...> as AppearsInFromClause<...>>::Count == Once`
    //~| ERROR: the trait bound `ReturningQuerySource<..., ...>: TableNotEqual<...>` is not satisfied
    //~| ERROR: the trait bound `ReturningQuerySource<..., ...>: Table` is not satisfied

    update(users::table)
        .set(users::columns::name.eq("Bill"))
        .returning(non_users::columns::noname);
    //~^ ERROR: `non_users::columns::noname` cannot appear in the `RETURNING` clause of a `UpdateStmt` on `users::table`
    //~| ERROR: type mismatch resolving `<ReturningQuerySource<..., ...> as AppearsInFromClause<...>>::Count == Once`
    //~| ERROR: the trait bound `ReturningQuerySource<..., ...>: TableNotEqual<...>` is not satisfied
    //~| ERROR: the trait bound `ReturningQuerySource<UpdateStmt, users::table>: Table` is not satisfied
}
