extern crate diesel;

use diesel::dsl::*;
use diesel::*;

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&mut connection);

    let mut connection = MysqlConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&mut connection);
}
