extern crate diesel;

use diesel::*;
use diesel::dsl::sql;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();
    let select_count = users::table.select(sql::<sql_types::BigInt>("COUNT(*)"));
    let count = select_count.get_result::<String>(&mut connection).unwrap();
}
