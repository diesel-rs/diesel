#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}



fn main() {
    let connection = MysqlConnection::establish("").unwrap();
    users::table.offset(42).get_result::<(i32, String)>(&connection);

    users::table.offset(42).into_boxed().get_result::<(i32, String)>(&connection);
}
