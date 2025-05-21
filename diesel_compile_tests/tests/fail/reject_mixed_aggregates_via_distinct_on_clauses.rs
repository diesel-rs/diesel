extern crate diesel;

use diesel::*;

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        hair_color -> Text,
    }
}

fn main() {
    let mut conn = PgConnection::establish("…").unwrap();

    // that one is ok
    let _ = posts::table
        .group_by(posts::user_id)
        .select(posts::user_id)
        .distinct_on(posts::user_id)
        .get_result::<i32>(&mut conn);

    // these should fail
    let _ = posts::table
        .group_by(posts::user_id)
        .distinct_on(posts::id) // error
        .select(posts::user_id)
        .get_results::<i32>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::id)
        .group_by(posts::user_id)
        .select(posts::user_id) // error
        .get_results::<i32>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::user_id)
        .select(dsl::count(posts::id)) // error
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .select(dsl::count(posts::id))
        .distinct_on(posts::user_id) // error
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::user_id)
        .count() // error
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .count()
        .distinct_on(posts::user_id) // error
        .get_result::<i64>(&mut conn);
}
