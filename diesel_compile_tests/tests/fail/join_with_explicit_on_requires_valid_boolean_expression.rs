extern crate diesel;

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

allow_tables_to_appear_in_same_query!(users, posts);
allow_tables_to_appear_in_same_query!(users, comments);
allow_tables_to_appear_in_same_query!(posts, comments);

fn main() {
    // Sanity check, make sure valid joins compile
    let _ = users::table.inner_join(posts::table.on(users::id.eq(posts::id)));
    // Invalid, references column that isn't being queried
    let _ = users::table.inner_join(posts::table.on(users::id.eq(comments::id)));
    // Invalid, type is not boolean
    let _ = users::table.inner_join(posts::table.on(users::id));
}
