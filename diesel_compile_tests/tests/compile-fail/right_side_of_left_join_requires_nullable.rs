#[macro_use] extern crate diesel;

use diesel::*;
use diesel::sql_types::Text;

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
sql_function!(fn lower(x: Text) -> Text);

fn main() {
}

fn direct_joins() {
    let join = users::table.left_outer_join(posts::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0277
}

fn nested_outer_joins_left_associative() {

    let join = users::table.left_outer_join(posts::table).left_outer_join(pets::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0277
}

fn nested_mixed_joins_left_associative() {
    let join = users::table.left_outer_join(posts::table).inner_join(pets::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0277
}

fn nested_outer_joins_right_associative() {
    let join = pets::table.left_outer_join(users::table.left_outer_join(posts::table));

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0277
}

fn nested_mixed_joins_right_associative() {
    let join = pets::table.inner_join(users::table.left_outer_join(posts::table));

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0277
}
