#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    let select_count = users::table.select_sql::<types::BigInt>("COUNT(*)");
    let count = select_count.get_result::<String>(&connection).unwrap();
    //~^ ERROR E0277
}
