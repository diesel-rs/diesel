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
        .distinct_on(posts::id)
        //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ...>: DistinctOnDsl<_>` is not satisfied
        .select(posts::user_id)
        .get_results::<i32>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::id)
        .group_by(posts::user_id)
        .select(posts::user_id)
        //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ...>: SelectDsl<_>` is not satisfied
        .get_results::<i32>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::user_id)
        .select(dsl::count(posts::id))
        //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .select(dsl::count(posts::id))
        .distinct_on(posts::user_id)
        //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .distinct_on(posts::user_id)
        .count()
        //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ...>: SelectDsl<...>` is not satisfied
        .get_result::<i64>(&mut conn);

    let _ = posts::table
        .count()
        .distinct_on(posts::user_id)
        //~^ ERROR: the trait bound `diesel::expression::is_aggregate::No: MixedAggregates<diesel::expression::is_aggregate::Yes>` is not satisfied
        .get_result::<i64>(&mut conn);
}
