#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::*;

fn main() {
    let connection = PgConnection::establish("").unwrap();
    select(array((1, 3))).get_result::<Vec<i32>>(&connection).unwrap();
    select(array((1f64, 3f64))).get_result::<Vec<f64>>(&connection).unwrap();

    select(array((1, 3f64))).get_result::<Vec<i32>>(&connection).unwrap();
    //~^ ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277

    select(array((1, 3f64))).get_result::<Vec<f64>>(&connection).unwrap();
    //~^ ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
    //~| ERROR E0277
}
