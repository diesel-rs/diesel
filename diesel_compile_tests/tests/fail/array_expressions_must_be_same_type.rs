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
        //~^ ERROR: the trait bound `f64: diesel::Expression` is not satisfied
        //~| ERROR: cannot select `f64` from `NoFromClause`
        //~| ERROR: the trait bound `f64: ValidGrouping<()>` is not satisfied
        .get_result::<Vec<i32>>(&mut connection)
        //~^ ERROR: cannot select `f64` from `NoFromClause`
        //~| ERROR: the trait bound `f64: ValidGrouping<()>` is not satisfied
        //~| ERROR: the trait bound `f64: QueryId` is not satisfied
        //~| ERROR: `f64` is no valid SQL fragment for the `Pg` backend
        .unwrap();
    select(array((1, 3f64)))
        //~^ ERROR: the trait bound `{integer}: diesel::Expression` is not satisfied
        //~| ERROR: cannot select `{integer}` from `NoFromClause`
        //~| ERROR: the trait bound `{integer}: ValidGrouping<()>` is not satisfied
        .get_result::<Vec<f64>>(&mut connection)
        //~^ ERROR: cannot select `{integer}` from `NoFromClause`
        //~| ERROR: the trait bound `{integer}: ValidGrouping<()>` is not satisfied
        //~| ERROR: the trait bound `{integer}: QueryId` is not satisfied
        //~| ERROR: `{integer}` is no valid SQL fragment for the `Pg` backend
        .unwrap();
}
