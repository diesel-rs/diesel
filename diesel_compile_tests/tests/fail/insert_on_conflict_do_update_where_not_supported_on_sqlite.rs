extern crate diesel;

use diesel::query_dsl::methods::FilterDsl;
use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();

    diesel::insert_into(users::table)
        .values(users::id.eq(42))
        .on_conflict(users::id)
        .do_update()
        .set(users::id.eq(42))
        .filter(users::id.eq(45))
        .execute(&mut connection)
        //~^ ERROR: the trait bound `sqlite::backend::SqliteOnConflictClause: SupportsOnConflictClauseWhere` is not satisfied
        .unwrap();
}
