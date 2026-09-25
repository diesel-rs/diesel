extern crate diesel;

use diesel::dsl::count_star;
use diesel::*;

table! {
    parents {
        id -> Integer,
        name -> Text,
    }
}

table! {
    children {
        id -> Integer,
        parent_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(parents, children);

// The grouping check of a subselect only inspects its `WHERE` clause, so every
// other clause must keep rejecting columns of the outer query until it is
// checked against the outer `GROUP BY` clause too
fn main() {
    let _ = parents::table.group_by(parents::id).select((
        parents::id,
        children::table
            .filter(children::parent_id.eq(parents::id))
            .select(parents::name)
            //~^ ERROR: cannot select `parents::columns::name` from `children::table`
            .single_value(),
            //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ...>: LimitDsl` is not satisfied
            //~| ERROR: cannot select `parents::columns::name` from `children::table`
    ));

    let _ = parents::table.group_by(parents::id).select((
        //~^ ERROR: the trait bound `(id, Exists<...>): Expression` is not satisfied
        parents::id,
        dsl::exists(
            children::table
                .filter(children::id.eq(parents::id))
                .select((parents::name, children::id)),
            //~^ ERROR: cannot select `parents::columns::name` from `children::table`
            //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
        ),
    ));

    let _ = parents::table.group_by(parents::id).select((
        parents::id,
        children::table
            .filter(children::parent_id.eq(parents::id))
            .order_by(parents::name)
            //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
            .select(children::id)
            .single_value(),
    ));

    let _ = parents::table.group_by(parents::id).select((
        parents::id,
        children::table
            .filter(children::parent_id.eq(parents::id))
            .group_by(parents::name)
            //~^ ERROR: type mismatch resolving `<FromClause<table> as AppearsInFromClause<table>>::Count == Once`
            .select(count_star())
            .single_value(),
    ));

    let _ = parents::table.group_by(parents::id).select((
        parents::id,
        children::table
            .filter(children::parent_id.eq(parents::id))
            .group_by(children::id)
            .having(parents::name.eq(""))
            //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
            .select(children::id)
            .single_value(),
    ));

    let _ = parents::table.group_by(parents::id).select((
        parents::id,
        children::table
            .filter(children::parent_id.eq(parents::id))
            .distinct_on(parents::name)
            //~^ ERROR: cannot select `parents::columns::name` from `children::table`
            .select(children::id)
            .single_value(),
    ));
}
