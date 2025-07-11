extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        title -> Text,
        user_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);
joinable!(posts -> users(user_id));

fn main() {
    let mut conn = SqliteConnection::establish("").unwrap();

    let q = users::table
        .inner_join(posts::table)
        .group_by((users::id, posts::user_id))
        .select((users::id, posts::user_id, diesel::dsl::count_star()));
    //~^ ERROR: the trait bound `posts::columns::user_id: IsContainedInGroupBy<users::columns::id>` is not satisfied
    //~| ERROR: he trait bound `users::columns::id: IsContainedInGroupBy<posts::columns::user_id>` is not satisfied
}
