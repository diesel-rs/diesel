//@error-in-other-file: evaluation of `<diesel::query_builder::where_clause::WhereClause<diesel::expression::grouped::Grouped<diesel::expression::operators::Lt<diesel::expression::functions::aggregate_expressions::AggregateExpression<diesel::expression::count::count_utils::count<diesel::sql_types::Integer, users::columns::id>, diesel::expression::functions::aggregate_expressions::prefix::NoPrefix, diesel::expression::functions::aggregate_expressions::aggregate_order::NoOrder, diesel::expression::functions::aggregate_expressions::aggregate_filter::NoFilter, diesel::expression::functions::aggregate_expressions::over_clause::OverClause>, diesel::expression::bound::Bound<diesel::sql_types::BigInt, i64>>>> as diesel::query_builder::QueryId>::IS_WINDOW_FUNCTION::{constant#0}` failed
// FIXME: Try to see if we can get a better error here
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();

    let _res = users::table
        .filter(dsl::count(users::id).over().lt(53))
        .count()
        .load::<i64>(&mut connection)
        .unwrap();
}
