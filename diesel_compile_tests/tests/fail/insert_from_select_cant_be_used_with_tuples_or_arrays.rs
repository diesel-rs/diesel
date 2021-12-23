extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    posts (user_id) {
        user_id -> Integer,
        title -> Text,
        body -> Nullable<Text>,
    }
}

fn main() {
    use self::users::dsl::*;
    use self::posts::dsl::*;
    let mut conn = PgConnection::establish("").unwrap();

    // Sanity check, valid query
    insert_into(posts)
        .values(users)
        .execute(&mut conn)
        .unwrap();

    insert_into(posts)
        .values(vec![users, users]);

    insert_into(posts)
        .values((users, users));
}
