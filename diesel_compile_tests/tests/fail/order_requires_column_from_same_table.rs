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
    let source = users::table.order(posts::id);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
