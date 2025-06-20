extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        name -> Text,
        content -> Text,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    let mut connection = PgConnection::establish("postgres://foo").unwrap();

    users::table
        .distinct_on(posts::id)
        //~^ ERROR: cannot select `posts::columns::id` from `users::table`
        .get_results(&mut connection);
    //~^ ERROR: the trait bound `(diesel::sql_types::Integer, diesel::sql_types::Text): SingleValue` is not satisfied

    posts::table
        .distinct_on((posts::name, users::name))
        //~^ ERROR: cannot select `users::columns::name` from `posts::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .get_result(&mut connection);
    //~^ ERROR: the trait bound `(diesel::sql_types::Integer, diesel::sql_types::Text, diesel::sql_types::Text): SingleValue` is not satisfied
}
