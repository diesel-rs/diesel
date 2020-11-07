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
        name -> Text,
        content -> Text,
    }
}

fn main() {
    let connection = PgConnection::establish("postgres://foo").unwrap();

    users::table.distinct_on(posts::id).get_results(&connection);

    posts::table.distinct_on((posts::name, users::name)).get_result(&connection);
}
