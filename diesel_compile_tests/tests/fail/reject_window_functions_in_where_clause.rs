//@error-in-other-file: evaluation panicked: Using window functions in WHERE clauses is not supported
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();

    let _res = users::table
        .filter(dsl::count(users::id).over().lt(53))
        .count()
        .load::<i64>(&mut connection)
        .unwrap();
}
