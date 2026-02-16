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

allow_tables_to_appear_in_same_query!(users, posts);

fn main() {
    let mut connection = PgConnection::establish("postgres://foo").unwrap();

    users::table
        .distinct_on(posts::id)
        //~^ ERROR: the trait bound `users::table: DistinctOnDsl<posts::columns::id>` is not satisfied
        .get_results(&mut connection);

    posts::table
        .distinct_on((posts::name, users::name))
        //~^ ERROR: the trait bound `table: DistinctOnDsl<(name, name)>` is not satisfied
        .get_result(&mut connection);
}
