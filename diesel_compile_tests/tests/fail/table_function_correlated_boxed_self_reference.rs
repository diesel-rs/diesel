extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

view! { json_each { key -> Integer, value -> Text, } }

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;
}

fn main() {
    let connection = &mut SqliteConnection::establish(":memory:").unwrap();
    json_each("[]")
        .filter(diesel::dsl::exists(
            json_each(json_each::value)
                .correlated()
                .select(json_each::key)
                .into_boxed::<Sqlite>(),
        ))
        .load::<(i32, String)>(connection);
        //~^ ERROR: type mismatch resolving
}
