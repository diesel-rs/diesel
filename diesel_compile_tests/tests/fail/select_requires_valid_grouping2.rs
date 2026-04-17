extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
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
        text -> Text,
        post_id -> Integer,
    }
}

joinable!(comments -> posts (post_id));
joinable!(posts -> users (user_id));

allow_tables_to_appear_in_same_query!(users, posts, comments);
allow_columns_to_appear_in_same_group_by_clause!(
    posts::title,
    users::id,
    posts::user_id,
    users::name,
    posts::id,
    users::hair_color
);

fn main() {
    let source = users::table
        .group_by(users::name)
        .select((users::name, users::id));
    //~^ ERROR: type mismatch resolving `<name as IsContainedInGroupBy<id>>::Output == Yes`

    let source = users::table
        .inner_join(posts::table.inner_join(comments::table))
        .group_by((users::id, posts::id))
        .select((users::all_columns, posts::all_columns, comments::id));
    //~^ ERROR: the trait bound `id: IsContainedInGroupBy<id>` is not satisfied
    //~| ERROR: the trait bound `id: IsContainedInGroupBy<id>` is not satisfied
}
