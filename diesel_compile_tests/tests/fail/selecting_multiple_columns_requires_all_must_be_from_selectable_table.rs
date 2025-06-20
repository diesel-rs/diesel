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
        title -> VarChar,
        user_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    let stuff = users::table.select((posts::id, posts::user_id));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: cannot select `posts::columns::user_id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
    let stuff = users::table.select((posts::id, users::name));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
