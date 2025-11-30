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
    // Sanity check: Valid update
    update(users::table).filter(users::id.eq(1));

    update(users::table.filter(posts::id.eq(1)));
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ..., ...>: IntoUpdateTarget` is not satisfied
    //~| ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ..., ...>: IntoUpdateTarget` is not satisfied

    update(users::table).filter(posts::id.eq(1));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    update(users::table)
        .set(users::id.eq(1))
        .filter(posts::id.eq(1));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
