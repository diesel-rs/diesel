extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

view! {
    series {
        value -> Integer,
    }
}

view! {
    pairs {
        key -> Text,
        value -> Integer,
    }
}

view! {
    other_schema.listed {
        value -> Integer,
    }
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(series)]
    fn generate_series(start: Integer, stop: Integer) -> Integer;
    //~^ ERROR: mismatched types
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(pairs, base_type)]
    //~^ ERROR: the relation of a `base_type` function must declare exactly one column, found `(pairs::columns::key, pairs::columns::value)`
    fn json_pairs(json: Text) -> pairs::SqlType;
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(listed)]
    //~^ ERROR: a function call in `FROM` must be named after a `view!` without a schema
    fn listed() -> listed::SqlType;
}

#[declare_sql_function]
extern "SQL" {
//~^ ERROR: invalid ABI: found `SQL`
    #[query_source(series, ordinality)]
    //~^ ERROR: the correct format is `#[query_source(relation)]` or `#[query_source(relation, base_type)]`
    //~| ERROR: cannot find attribute `query_source` in this scope
    fn ordinal_series(stop: Integer) -> series::SqlType;
}

#[declare_sql_function]
extern "SQL" {
    #[aggregate]
    #[query_source(series)]
    //~^ ERROR: `#[query_source]` cannot be combined with `#[aggregate]` or `#[window]`
    fn aggregate_series(stop: Integer) -> series::SqlType;
}

#[declare_sql_function]
extern "SQL" {
    #[variadic(1)]
    #[query_source(series)]
    //~^ ERROR: `#[query_source]` cannot be combined with `#[variadic]`
    fn variadic_series(stop: Integer) -> series::SqlType;
}

fn main() {}
