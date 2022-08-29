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
        title -> Text,
        user_id -> Integer,
    }
}

table! {
    comments {
        id -> Integer,
        user_id -> Integer,
        post_id -> Integer,
        name -> Text,
    }
}

joinable!(posts -> users (user_id));
joinable!(comments -> users (user_id));
joinable!(comments -> posts (post_id));
allow_tables_to_appear_in_same_query!(posts, users, comments);

fn main() {}

fn invalid_inner_joins() {
    // This is a valid join
    let _ = users::table.inner_join(posts::table);

    // This fails, because we join the same table more than once
    let _ = users::table.inner_join(posts::table.inner_join(users::table));

    // It also fails if we use an explicit on clause
    let _ = users::table.inner_join(posts::table.inner_join(users::table.on(posts::user_id.eq(users::id))));

    // Also if we put the on clause on the first join
    let _ = users::table.inner_join(posts::table.on(users::id.eq(posts::user_id)).inner_join(users::table));

    // it also fails if we join to another subjoin
    let _ = users::table.inner_join(comments::table).inner_join(posts::table.inner_join(comments::table));
}

fn invalid_left_joins() {
    // This is a valid join
    let _ = users::table.left_join(posts::table);

    // This fails, because we join the same table more than once
    let _ = users::table.left_join(posts::table.left_join(users::table));

    // It also fails if we use an explicit on clause
    let _ = users::table.left_join(posts::table.left_join(users::table.on(posts::user_id.eq(users::id))));

    // Also if we put the on clause on the first join
    let _ = users::table.left_join(posts::table.on(users::id.eq(posts::user_id)).left_join(users::table));

    // it also fails if we join to another subjoin
    let _ = users::table.left_join(comments::table).left_join(posts::table.left_join(comments::table));
}
