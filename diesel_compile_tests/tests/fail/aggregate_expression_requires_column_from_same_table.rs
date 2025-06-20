extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    use diesel::dsl::*;
    let source = users::table.select(sum(posts::id));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
    let source = users::table.select(avg(posts::id));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
    let source = users::table.select(max(posts::id));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
    let source = users::table.select(min(posts::id));
    //~^ ERROR: cannot select `posts::columns::id` from `users::table`
    //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
