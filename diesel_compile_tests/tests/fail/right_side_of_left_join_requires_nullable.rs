extern crate diesel;

use diesel::sql_types::Text;
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
        title -> Text,
        user_id -> Integer,
    }
}

table! {
    pets {
        id -> Integer,
        user_id -> Integer,
        name -> Text,
    }
}

joinable!(posts -> users (user_id));
joinable!(pets -> users (user_id));
allow_tables_to_appear_in_same_query!(posts, users, pets);

#[declare_sql_function]
extern "SQL" {
    fn lower(x: Text) -> Text;
}

fn main() {}

fn direct_joins() {
    let join = users::table.left_outer_join(posts::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: annot select `posts::columns::title` from `users::table`
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: annot select `posts::columns::title` from `users::table`
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
    //~| ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
}

fn nested_outer_joins_left_associative() {
    let join = users::table
        .left_outer_join(posts::table)
        .left_outer_join(pets::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
    //~| ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
}

fn nested_mixed_joins_left_associative() {
    let join = users::table
        .left_outer_join(posts::table)
        .inner_join(pets::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
    //~| ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
}

fn nested_outer_joins_right_associative() {
    let join = pets::table.left_outer_join(users::table.left_outer_join(posts::table));

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR: type mismatch resolving `<SelectStatement<...> as AppearsInFromClause<...>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `pets::table`
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR: type mismatch resolving `<SelectStatement<...> as AppearsInFromClause<...>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `pets::table`
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
    //~| ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
}

fn nested_mixed_joins_right_associative() {
    let join = pets::table.inner_join(users::table.left_outer_join(posts::table));

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Never`
    //~| ERROR: cannot select `posts::columns::title` from `users::table`
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
    //~| ERROR: the trait bound `Nullable<title>: AsExpression<Text>` is not satisfied
}
