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

    users::table
        .inner_join(json_each(users::name).on(json_each::value.eq(users::name)))
        //~^ ERROR: the method `on` exists for struct `FunctionCall<view, json_each<name>>`, but its trait bounds were not satisfied
        .select(json_each::value)
        .load::<String>(conn)
        .unwrap();

    json_each(users::name)
        .select(json_each::value)
        //~^ ERROR: the trait bound `FunctionCall<view, json_each<name>>: SelectDsl<_>` is not satisfied
        .load::<String>(conn)
        .unwrap();
}
