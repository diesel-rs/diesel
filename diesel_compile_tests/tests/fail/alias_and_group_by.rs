extern crate diesel;

use diesel::alias;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let conn = &mut PgConnection::establish("…").unwrap();
    let user_alias = alias!(users as user1);

    // allowed as this groups by the same field
    user_alias
        .group_by(user_alias.field(users::name))
        .select(user_alias.field(users::name))
        .execute(conn)
        .unwrap();

    // allowed as this groups by the primary key
    user_alias
        .group_by(user_alias.field(users::id))
        .select(user_alias.field(users::name))
        .execute(conn)
        .unwrap();

    // fails as this groups by an incompatible field
    user_alias
        .group_by(user_alias.field(users::name))
        .select(user_alias.field(users::id))
        .execute(conn)
        .unwrap();
}
