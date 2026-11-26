extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

table! { users { id -> Integer, name -> Text, } }
view! { json_each { value -> Text, } }
allow_tables_to_appear_in_same_query!(users, json_each);

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;
}

fn main() {
    let connection = &mut SqliteConnection::establish(":memory:").unwrap();
    users::table
        .filter("b".into_sql::<Text>().eq_any(
            json_each(users::name)
                .correlated()
                .inner_join(users::table.on(users::id.eq(1)))
                .select(json_each::value)
                .into_boxed::<Sqlite>(),
        ))
        .load::<(i32, String)>(connection);
        //~^ ERROR: type mismatch resolving
}
