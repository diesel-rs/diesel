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

allow_tables_to_appear_in_same_query!(users, json_each);

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;
}

fn main() {
    let conn = &mut SqliteConnection::establish("").unwrap();

    json_each("[]")
        .select(users::name)
        //~^ ERROR: cannot select `users::columns::name` from `FunctionCall<view, json_each<Bound<Text, &str>>>`
        .load::<String>(conn)
        //~^ ERROR: cannot select `users::columns::name` from `FunctionCall<view, json_each<Bound<Text, &str>>>`
        .unwrap();
}
