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
}

fn main() {
    let conn = &mut MysqlConnection::establish("").unwrap();

    json_each("[]")
        .select(json_each::value)
        .load::<String>(conn)
        //~^ ERROR: `FunctionCall<view, json_each<Bound<Text, &str>>>` is no valid SQL fragment for the `Mysql` backend
        .unwrap();
}
