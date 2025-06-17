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
    let _ = users::table.inner_join(posts::table);
    //~^ ERROR: the trait bound `users::table: JoinTo<posts::table>` is not satisfied
    let _ = users::table.left_outer_join(posts::table);
    //~^ ERROR: the trait bound `users::table: JoinTo<posts::table>` is not satisfied

    // Sanity check to make sure the error is when users
    // become involved
    let join = posts::table.inner_join(comments::table);
    let _ = users::table.inner_join(join);
    //~^ ERROR: the trait bound `posts::table: JoinTo<users::table>` is not satisfied
}
