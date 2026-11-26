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
        //~^ ERROR: the trait bound `Join<table, _, Inner>: AppearsInFromClause<view>` is not satisfied
        //~| ERROR: the trait bound `Join<table, _, Inner>: AppearsInFromClause<table>` is not satisfied
        //~| ERROR: function arguments cannot reference columns in an ordinary join
        .select(json_each::value)
        //~^ ERROR: no method named `select` found for struct `SelectStatement<From, Select, Distinct, Where, Order, LimitOffset, GroupBy, Having, Locking>` in the current scope
        .load::<String>(conn)
        .unwrap();

    users::table.left_join(json_each(users::name).on(json_each::value.eq(users::name)));
    //~^ ERROR: function arguments cannot reference columns in an ordinary join
    //~| ERROR: the trait bound `Join<table, _, _>: AppearsInFromClause<view>` is not satisfied
    //~| ERROR: the trait bound `Join<table, _, _>: AppearsInFromClause<table>` is not satisfied

    json_each(users::name)
        .select(json_each::value)
        //~^ ERROR: the trait bound `FunctionCall<view, json_each<name>>: SelectDsl<_>` is not satisfied
        .load::<String>(conn)
        .unwrap();
}
