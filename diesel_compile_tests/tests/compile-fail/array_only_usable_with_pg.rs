#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::*;

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&connection);
    //~^ ERROR E0271
    //~| ERROR E0277

    let connection = MysqlConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&connection);
    //~^ ERROR E0271
    //~| ERROR E0277
}
