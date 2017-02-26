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
select_column_workaround!(posts -> users (id, title, user_id));
select_column_workaround!(users -> posts (id, name));
sql_function!(lower, lower_t, (x: Text) -> Text);

fn main() {
    let conn = PgConnection::establish("some url").unwrap();
    let join = users::table.left_outer_join(posts::table);

    // Valid
    let _ = join.select(posts::title.nullable());
    // FIXME: This doesn't compile but we want it to
    // Valid -- NULL to a function will return null
    let _ = join.select(lower(posts::title).nullable()); //~ ERROR E0271
    // Invalid, only Nullable<title> is selectable
    let _ = join.select(lower(posts::title)); //~ ERROR E0271
    // Invalid, Nullable<title> is selectable, but lower expects not-null
    let _ = join.select(lower(posts::title.nullable())); //~ ERROR E0271
}
