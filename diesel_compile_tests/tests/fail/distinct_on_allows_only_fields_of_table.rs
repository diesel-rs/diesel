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

    posts::table
        .distinct_on((posts::name, users::name))
        //~^ ERROR: cannot select `users::columns::name` from `posts::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        .get_result(&mut connection);
}
