#[macro_use]
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
    use users::dsl::*;
    use posts::dsl::*;
    let conn = PgConnection::establish("").unwrap();

    // Sanity check, valid query
    insert_into(posts)
        .values(users)
        .execute(&conn)
        .unwrap();

    insert_into(posts)
        .values(vec![users, users]);
        //~^ ERROR E0277

    insert_into(posts)
        .values((users, users));
        //~^ ERROR E0271
}
