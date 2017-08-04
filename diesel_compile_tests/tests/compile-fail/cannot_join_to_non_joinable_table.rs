#[macro_use]
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

table! {
    comments {
        id -> Integer,
        post_id -> Integer,
    }
}

joinable!(comments -> posts (post_id));
enable_multi_table_joins!(users, posts);
enable_multi_table_joins!(users, comments);

fn main() {
    let _ = users::table.inner_join(posts::table);
    //~^ ERROR 0277
    let _ = users::table.left_outer_join(posts::table);
    //~^ ERROR 0277

    // Sanity check to make sure the error is when users
    // become involved
    let join = posts::table.inner_join(comments::table);
    let _ = users::table.inner_join(join);
    //~^ ERROR 0277
}
