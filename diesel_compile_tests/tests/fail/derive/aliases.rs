#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use diesel::query_builder::AsQuery;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        author -> Integer,
        title -> Text,
    }
}

table! {
    pets {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);
joinable!(posts -> users (author));

pub fn check(conn: &PgConnection) {
    let user_alias = alias!(users as users2);
    let post_alias = alias!(posts as posts2);

    // wrong fields

    user_alias.field(posts::id);

    // joining the same alias twice

    users::table
        .inner_join(post_alias)
        .inner_join(post_alias)
        .select(users::id)
        .load::<i32>(conn)
        .unwrap();

    user_alias
        .as_query()
        .select(users::id)
        .load::<i32>(conn)
        .unwrap();

    let user2_alias = alias!(users as user3);

    // dont't allow joins to not joinable tables
    pets::table
        .inner_join(user_alias)
        .select(pets::id)
        .load::<i32>(conn)
        .unwrap();
}

fn main() {}
