#[macro_use] extern crate diesel;

use diesel::prelude::*;

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

table! {
    comments {
        id -> Integer,
    }
}

enable_multi_table_joins!(users, posts);
enable_multi_table_joins!(users, comments);
enable_multi_table_joins!(posts, comments);

fn main() {
    // Sanity check, make sure valid joins compile
    let _ = users::table.inner_join(posts::table.on(users::id.eq(posts::id)));
    // Invalid, references column that isn't being queried
    let _ = users::table.inner_join(posts::table.on(users::id.eq(comments::id)));
    //~^ ERROR E0271
    // Invalid, type is not boolean
    let _ = users::table.inner_join(posts::table.on(users::id));
    //~^ ERROR E0271
}
