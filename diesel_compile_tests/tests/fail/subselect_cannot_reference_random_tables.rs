extern crate diesel;

use diesel::*;

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

allow_tables_to_appear_in_same_query!(users, posts, comments);

fn main() {
    use diesel::dsl::{any, exists};

    let mut conn = PgConnection::establish("").unwrap();

    let _ = users::table
        .filter(users::id.eq_any(posts::table.select(posts::id).filter(comments::id.eq(1))))
        .load::<(i32,)>(&mut conn);

    let _ = users::table
        .filter(users::id.eq(any(
            posts::table.select(posts::id).filter(comments::id.eq(1)),
        )))
        .load::<(i32,)>(&mut conn);

    let _ = users::table
        .filter(exists(posts::table.filter(comments::id.eq(1))))
        .load::<(i32,)>(&mut conn);
}
