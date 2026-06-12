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
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    use self::users::dsl::*;

    let command = update(users).set(posts::title.eq("Hello"));
    //~^ ERROR: type mismatch resolving `<Grouped<...> as AsChangeset>::Target == table`
    let command = update(users).set(name.eq(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
