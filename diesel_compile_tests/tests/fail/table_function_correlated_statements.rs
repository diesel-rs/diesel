extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

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
    json_each(users::name).correlated().load::<(String,)>(connection);
    //~^ ERROR: the method `load` exists
    json_each(users::name).correlated().into_boxed::<diesel::sqlite::Sqlite>().load::<(String,)>(connection);
    //~^ ERROR: the method `load` exists
    json_each(users::name).correlated().into_boxed_clone::<diesel::sqlite::Sqlite>().load::<(String,)>(connection);
    //~^ ERROR: the method `load` exists
    json_each(users::name).correlated().union(json_each("[]"));
    //~^ ERROR: the method `union` exists for struct `CorrelatedSelect<SelectStatement<_>, _>`, but its trait bounds were not satisfied
    users::table.inner_join(json_each(users::name).correlated().on(json_each::value.eq(users::name)));
    //~^ ERROR: the method `on` exists for struct `CorrelatedSelect<SelectStatement<_>, _>`, but its trait bounds were not satisfied
}

fn combined_right_hand_side() {
    json_each("[]").union(json_each(users::name).correlated());
    //~^ ERROR: the trait bound `CorrelatedSelect<SelectStatement<_>, _>: Query` is not satisfied
}
