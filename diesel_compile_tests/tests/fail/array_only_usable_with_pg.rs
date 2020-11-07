extern crate diesel;

use diesel::dsl::*;
use diesel::*;

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&connection);

    let connection = MysqlConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&connection);
}
