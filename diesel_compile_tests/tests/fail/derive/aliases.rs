#[macro_use]
extern crate diesel;

use diesel::prelude::*;

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

allow_tables_to_appear_in_same_query!(users, posts, pets);
joinable!(posts -> users (author));

pub fn check(conn: &mut PgConnection) {
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

    // Selecting the raw field on the aliased table
    user_alias.select(users::id).load::<i32>(conn).unwrap();

    let user2_alias = alias!(users as user3);

    // don't allow joins to not joinable tables
    pets::table
        .inner_join(user_alias)
        .select(pets::id)
        .load::<i32>(conn)
        .unwrap();

    // Check how error message looks when aliases to the same table are declared separately
    let post_alias_2 = alias!(posts as posts3);
    let posts = post_alias
        .inner_join(
            post_alias_2.on(post_alias
                .field(posts::author)
                .eq(post_alias_2.field(posts::author))),
        )
        .select((post_alias.field(posts::id), post_alias_2.field(posts::id)))
        .load::<(i32, i32)>(conn)
        .unwrap();
}

fn main() {}
