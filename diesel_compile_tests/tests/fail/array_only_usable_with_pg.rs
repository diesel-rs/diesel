extern crate diesel;

use diesel::dsl::*;
use diesel::*;

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&mut connection);
    //~^ ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`

    let mut connection = MysqlConnection::establish("").unwrap();
    select(array((1,))).get_result::<Vec<i32>>(&mut connection);
    //~^ ERROR: type mismatch resolving `<MysqlConnection as Connection>::Backend == Pg`
}
