extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

view! {
    json_each {
        value -> Text,
    }
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;
}

fn main() {
    let conn = &mut SqliteConnection::establish("").unwrap();

    users::table
        .select(json_each("[]"))
        //~^ ERROR: cannot select `FunctionCall<view, json_each<Bound<Text, &str>>>` from `users::table`
        //~| ERROR: the trait bound `FunctionCall<view, _>: ValidGrouping<()>` is not satisfied
        .load::<String>(conn)
        //~^ ERROR: cannot select `FunctionCall<view, json_each<Bound<Text, &str>>>` from `users::table`
        //~| ERROR: the trait bound `FunctionCall<view, _>: ValidGrouping<()>` is not satisfied
        .unwrap();
}
