extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let conn = &mut PgConnection::establish("…").unwrap();
    users::table
        .filter(users::id.eq(1).and(users::id).or(users::id))
        //~^ ERROR: `diesel::sql_types::Integer` is neither `diesel::sql_types::Bool` nor `diesel::sql_types::Nullable<Bool>`
        //~| ERROR: `diesel::sql_types::Integer` is neither `diesel::sql_types::Bool` nor `diesel::sql_types::Nullable<Bool>`
        .select(users::id)
        .execute(conn)
        .unwrap();
}
