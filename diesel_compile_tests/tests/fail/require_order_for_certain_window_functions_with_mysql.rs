extern crate diesel;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    let conn = &mut MysqlConnection::establish("").unwrap();
    let _ = users::table
        .select(dsl::rank().partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::dense_rank().partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::percent_rank().partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::cume_dist().partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lag(users::id).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lag_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lag_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lead(users::id).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lead_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied
    let _ = users::table
        .select(dsl::lead_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);
    //~^ ERROR: the trait bound `OverClause<...>: WindowFunctionFragment<..., ..., ...>` is not satisfied

    // These impls work as Sqlite and Postgres doesn't require the order clause
    let conn = &mut SqliteConnection::establish("").unwrap();
    let _ = users::table
        .select(dsl::rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::dense_rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::percent_rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::cume_dist().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag(users::id).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead(users::id).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);

    let conn = &mut PgConnection::establish("").unwrap();
    let _ = users::table
        .select(dsl::rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::dense_rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::percent_rank().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::cume_dist().partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag(users::id).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lag_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead(users::id).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead_with_offset(users::id, 42).partition_by(users::id))
        .execute(conn);
    let _ = users::table
        .select(dsl::lead_with_offset_and_default(users::id, 42, 42).partition_by(users::id))
        .execute(conn);
}
