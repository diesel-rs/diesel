extern crate diesel;

use diesel::expression::{is_aggregate, MixedAggregates, ValidGrouping};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable};

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn some_ungrouped_expression(
    something: bool,
) -> Box<dyn BoxableExpression<users::table, Pg, SqlType = Integer>> {
    if something {
        Box::new(5.into_sql::<Integer>())
    } else {
        Box::new(users::id)
    }
}

fn maybe_grouped<GB>(
    something: bool,
) -> Box<
    dyn BoxableExpression<
        users::table,
        Pg,
        GB,
        <users::id as ValidGrouping<GB>>::IsAggregate,
        SqlType = Integer,
    >,
>
where
    users::id: BoxableExpression<
            users::table,
            Pg,
            GB,
            <users::id as ValidGrouping<GB>>::IsAggregate,
            SqlType = Integer,
        > + ValidGrouping<GB>,
{
    if something {
        Box::new(5.into_sql::<Integer>())
    } else {
        Box::new(users::id)
    }
}

fn something_that_is_aggregate<GB>(
) -> Box<dyn BoxableExpression<users::table, Pg, GB, is_aggregate::Yes, SqlType = Nullable<Integer>>>
where
    diesel::dsl::count<users::id>: BoxableExpression<users::table, Pg, GB, is_aggregate::Yes>,
{
    Box::new(diesel::dsl::max(users::id))
}

fn main() {
    let mut conn = PgConnection::establish("connection_string").unwrap();

    // it's fine to pass this to some query without group clause
    users::table
        .select(some_ungrouped_expression(true))
        .load::<i32>(&mut conn);

    // this fails because there is explicitly no group by clause
    users::table
        .group_by(users::id)
        .select(some_ungrouped_expression(true))
        .load::<i32>(&mut conn);

    // it's fine to pass this to some query without group by clause
    // rustc should infer the correct bounds here
    users::table.select(maybe_grouped(false)).load::<i32>(&mut conn);

    // it's also fine to pass this to some query with a matching
    // group by clause
    users::table
        .group_by(users::id)
        .select(maybe_grouped(true))
        .load::<i32>(&mut conn);

    // this fails because of an incompatible group by clause
    users::table
        .group_by(users::name)
        .select(maybe_grouped(true))
        .load::<i32>(&mut conn);

    // aggregated expressions work to
    users::table
        .select(something_that_is_aggregate())
        .load::<Option<i32>>(&mut conn);

    // also with a group by clause
    users::table
        .group_by(users::name)
        .select(something_that_is_aggregate())
        .load::<Option<i32>>(&mut conn);

    // but we cannot mix a aggregated expression with an non aggregate one
    users::table
        .select((
            something_that_is_aggregate(),
            some_ungrouped_expression(false),
        ))
        .load::<(Option<i32>, i32)>(&mut conn);

    // using two potential aggregated expressions works
    users::table
        .group_by(users::id)
        .select((something_that_is_aggregate(), maybe_grouped(true)))
        .load::<(Option<i32>, i32)>(&mut conn);
}
