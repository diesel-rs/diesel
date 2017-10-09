#[macro_use]
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

allow_tables_to_appear_in_same_query!(users, posts);
allow_tables_to_appear_in_same_query!(users, comments);
allow_tables_to_appear_in_same_query!(posts, comments);

fn main() {
    use diesel::dsl::{any, exists};

    let conn = PgConnection::establish("").unwrap();

    let _ = LoadDsl::load::<(i32,)>(
    //~^ ERROR E0271
        users::table
            .filter(users::id.eq_any(posts::table.select(posts::id).filter(comments::id.eq(1)))),
        &conn,
    );

    let _ = LoadDsl::load::<(i32,)>(
    //~^ ERROR E0271
        users::table.filter(users::id.eq(any(
            posts::table.select(posts::id).filter(comments::id.eq(1)),
        ))),
        &conn,
    );

    let _ = LoadDsl::load::<(i32,)>(
    //~^ ERROR E0271
        users::table.filter(exists(posts::table.filter(comments::id.eq(1)))),
        &conn,
    );
}
