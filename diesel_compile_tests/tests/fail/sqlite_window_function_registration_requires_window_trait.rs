extern crate diesel;

use diesel::declare_sql_function;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::sqlite::{SqliteAggregateFunction, SqliteConnection};

#[declare_sql_function]
extern "SQL" {
    #[aggregate]
    #[window]
    fn my_win(x: Integer) -> Integer;
}

#[derive(Default)]
struct OnlyAggregate {
    sum: i32,
}

impl SqliteAggregateFunction<i32> for OnlyAggregate {
    type Output = i32;

    fn step(&mut self, expr: i32) {
        self.sum += expr;
    }

    fn finalize(aggregator: Option<Self>) -> Self::Output {
        aggregator.map(|a| a.sum).unwrap_or_default()
    }
}

fn main() {
    let conn = &mut SqliteConnection::establish(":memory:").unwrap();
    my_win_utils::register_impl::<OnlyAggregate, _>(conn).unwrap();
    //~^ ERROR: the trait bound `OnlyAggregate: SqliteWindowFunction<i32>` is not satisfied
}
