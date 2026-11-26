extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Integer;

table! { users { id -> Integer, name -> Text, } }
view! { series { value -> Integer, } }
allow_tables_to_appear_in_same_query!(users, series);

#[declare_sql_function]
extern "SQL" {
    #[query_source(series, base_type)]
    fn generate_series(start: Integer, stop: Integer) -> series::SqlType;
}

fn main() {
    let connection = &mut PgConnection::establish("").unwrap();
    users::table
        .group_by(users::name)
        .select((users::name, generate_series(1, users::id).correlated().select(series::value).single_value()))
        //~^ ERROR: type mismatch resolving `<name as IsContainedInGroupBy<id>>::Output == Yes`
        .load::<(String, Option<i32>)>(connection);
}
