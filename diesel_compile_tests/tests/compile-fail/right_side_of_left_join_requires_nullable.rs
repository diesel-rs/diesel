#[macro_use] extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;
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
sql_function!(lower, lower_t, (x: Text) -> Text);

fn main() {
    let conn = PgConnection::establish("some url").unwrap();
    let join = users::table.left_outer_join(posts::table);

    // Invalid, only Nullable<title> is selectable
    let _ = join.select(posts::title); //~ ERROR E0277
    // Valid
    let _ = join.select(posts::title.nullable());
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable());
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title)); //~ ERROR E0277
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable())); //~ ERROR E0271
}
