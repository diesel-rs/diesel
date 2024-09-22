extern crate diesel;

use diesel::alias;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);
allow_columns_to_appear_in_same_group_by_clause!(users::id, posts::id);

fn main() {
    let conn = &mut PgConnection::establish("..").unwrap();

    // this fails because `posts` is not part of the from clause
    users::table
        .group_by(posts::id)
        .select(users::id)
        .execute(conn)
        .unwrap();

    // order of select and group by does not matter for the error
    users::table
        .select(users::id)
        .group_by(posts::id)
        .execute(conn)
        .unwrap();

    let (user_alias, post_alias) = alias!(users as user1, posts as post1,);

    // this also fails if we use aliases
    user_alias
        .group_by(posts::id)
        .select(user_alias.field(users::id))
        .execute(conn)
        .unwrap();

    users::table
        .group_by(post_alias.field(posts::id))
        .select(users::id)
        .execute(conn)
        .unwrap();

    user_alias
        .group_by(post_alias.field(posts::id))
        .select(user_alias.field(users::id))
        .execute(conn)
        .unwrap();

    user_alias
        .select(user_alias.field(users::id))
        .group_by(posts::id)
        .execute(conn)
        .unwrap();

    users::table
        .select(users::id)
        .group_by(post_alias.field(posts::id))
        .execute(conn)
        .unwrap();

    user_alias
        .select(user_alias.field(users::id))
        .group_by(post_alias.field(posts::id))
        .execute(conn)
        .unwrap();
}
