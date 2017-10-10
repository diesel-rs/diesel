#[macro_use] extern crate diesel;

use diesel::*;
use diesel::types::Text;

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

joinable!(posts -> users (user_id));
allow_tables_to_appear_in_same_query!(posts, users);
sql_function!(lower, lower_t, (x: Text) -> Text);

fn main() {
    let conn = PgConnection::establish("some url").unwrap();
    let join = users::table.left_outer_join(posts::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title);
    //~^ ERROR E0271
    //~| ERROR E0271
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title));
    //~^ ERROR E0271
    //~| ERROR E0271
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable()));
    //~^ ERROR E0271
    //~| ERROR E0271
}
