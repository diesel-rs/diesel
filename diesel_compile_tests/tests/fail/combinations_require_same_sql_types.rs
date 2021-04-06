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
allow_tables_to_appear_in_same_query!(comments, posts, users);

fn main() {
    let _ = users::table.union(comments::table);

    // Sanity check to make sure the error is when comments
    // become involved
    let union = users::table.union(posts::table);
    let _ = union.union(comments::table);
}
