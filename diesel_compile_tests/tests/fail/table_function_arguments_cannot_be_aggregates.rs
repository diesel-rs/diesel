extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

view! {
    json_each {
        value -> Text,
    }
}

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;

    #[aggregate]
    fn group_concat(value: Text) -> Text;
}

fn main() {
    let conn = &mut SqliteConnection::establish("").unwrap();

    json_each(group_concat("[]"))
        .select(json_each::value)
        //~^ ERROR: the trait bound `FunctionCall<view, json_each<_>>: SelectDsl<_>` is not satisfied
        .load::<String>(conn)
        .unwrap();
}
