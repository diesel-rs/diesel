#[macro_use]
extern crate yaqb;

use yaqb::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

fn main() {
    let connection = Connection::establish("").unwrap();
    let select_count = users::table.select_sql::<types::BigInt>("COUNT(*)");
    let count = connection.query_one::<_, String>(select_count).unwrap();
    //~^ ERROR E0277
}
