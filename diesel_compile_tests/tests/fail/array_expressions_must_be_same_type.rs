extern crate diesel;

use diesel::dsl::*;
use diesel::*;

fn main() {
    let mut connection = PgConnection::establish("").unwrap();
    select(array((1, 3)))
        .get_result::<Vec<i32>>(&mut connection)
        .unwrap();
    select(array((1f64, 3f64)))
        .get_result::<Vec<f64>>(&mut connection)
        .unwrap();

    select(array((1, 3f64)))
        .get_result::<Vec<i32>>(&mut connection)
        .unwrap();
    select(array((1, 3f64)))
        .get_result::<Vec<f64>>(&mut connection)
        .unwrap();
}
