extern crate diesel;

use diesel::prelude::*;
use diesel::sql_types::Text;

table! { users { id -> Integer, name -> Text, } }
table! { posts { id -> Integer, title -> Text, } }
view! { json_each { value -> Text, } }
allow_tables_to_appear_in_same_query!(users, posts, json_each);

#[declare_sql_function]
extern "SQL" {
    #[query_source(json_each)]
    fn json_each(json: Text) -> json_each::SqlType;
}

fn main() {
    users::table.inner_join_lateral(json_each(posts::title).on(json_each::value.eq(users::name)));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    users::table
        .inner_join_lateral(json_each(posts::title).on(json_each::value.eq(users::name)))
        //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .inner_join(posts::table.on(posts::id.eq(users::id)));

    users::table.inner_join(
        posts::table
            .inner_join_lateral(json_each(users::name).on(json_each::value.eq(posts::title)))
            //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
            .on(posts::id.eq(users::id)),
    );
}
