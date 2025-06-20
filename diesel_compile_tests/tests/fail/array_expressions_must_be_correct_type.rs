extern crate diesel;

use diesel::dsl::*;
use diesel::*;

fn main() {
    let mut connection = PgConnection::establish("").unwrap();
    select(array((1, 3))).get_result::<Vec<i32>>(&mut connection);
    select(array((1f64, 3f64))).get_result::<Vec<i32>>(&mut connection);
    //~^ ERROR: cannot select `f64` from `NoFromClause`
    //~| ERROR: the trait bound `f64: ValidGrouping<()>` is not satisfied
    //~| ERROR: the trait bound `f64: QueryId` is not satisfied
    //~| ERROR: `f64` is no valid SQL fragment for the `Pg` backend
    //~| ERROR: the trait bound `f64: diesel::Expression` is not satisfied
    //~| ERROR: cannot select `f64` from `NoFromClause`
    //~| ERROR: the trait bound `f64: ValidGrouping<()>` is not satisfied
    //~| ERROR: cannot convert `(f64, f64)` into an expression of type `Array<diesel::sql_types::Integer>`
    select(array((1f64, 3f64))).get_result::<Vec<f64>>(&mut connection);
}
