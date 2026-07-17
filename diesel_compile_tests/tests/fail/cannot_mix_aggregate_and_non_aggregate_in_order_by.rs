extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

table! {
    parent {
        id -> Integer,
        name -> Text,
    }
}

table! {
    child {
        id -> Integer,
        parent_id -> Integer,
        value -> Text,
    }
}

joinable!(child -> parent (parent_id));
allow_tables_to_appear_in_same_query!(parent, child);

fn main() {
    use self::users::dsl::*;
    use diesel::dsl::max;

    let conn = &mut SqliteConnection::establish("…").unwrap();

    // -------------------------------------------------------------------------
    // Without GROUP BY — mixing aggregate and non-aggregate is always invalid
    // -------------------------------------------------------------------------

    // aggregate SELECT + non-aggregate ORDER BY
    let _ = users.select(max(id)).order_by(name);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate SELECT + aggregate ORDER BY
    let _ = users.select(id).order_by(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // aggregate SELECT + aggregate ORDER BY + non-aggregate then_order_by
    let _ = users.select(max(id)).order_by(max(id)).then_order_by(name);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate ORDER BY first, then aggregate SELECT (issue #3815)
    let _ = users.order_by(name).select(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate ORDER BY first, then .count() (issue #3815)
    let _ = users.order_by(name).count();
    //~^ ERROR: SelectDsl

    // aggregate ORDER BY without explicit SELECT (default non-aggregate select, no GROUP BY)
    let _ = users.order_by(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate SELECT + aggregate then_order_by (ThenOrderDsl→NoOrderClause path)
    let _ = users.select(id).then_order_by(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // non-aggregate ORDER BY + aggregate then_order_by (ThenOrderDsl→existing OrderClause path)
    let _ = users.order_by(name).then_order_by(max(id));
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // -------------------------------------------------------------------------
    // With GROUP BY — non-grouped non-aggregate column in ORDER BY is invalid
    // -------------------------------------------------------------------------

    // non-grouped column in order_by, default select, order before select
    let _ = users.group_by(name).order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // non-grouped column in order_by, order before explicit select
    let _ = users.group_by(name).order_by(id).select(name);
    //~^ ERROR: IsContainedInGroupBy
    //~| ERROR: SelectDsl

    // non-grouped column in order_by, select called first
    // When select is first, S::Selection satisfies ValidGrouping, so the error
    // narrows to the specific column's IsContainedInGroupBy constraint
    let _ = users.group_by(name).select((name, max(id))).order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // valid order_by (aggregate), then non-grouped column in then_order_by; select first
    let _ = users
        .group_by(name)
        .select((name, max(id)))
        .order_by(max(id))
        .then_order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // valid order_by (grouped column), then non-grouped column in then_order_by; select first
    let _ = users
        .group_by(name)
        .select((name, max(id)))
        .order_by(name)
        .then_order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // non-grouped then_order_by after grouped col order_by, order-before-select
    let _ = users.group_by(name).order_by(name).then_order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // non-grouped then_order_by after aggregate order_by, order-before-select
    let _ = users.group_by(name).order_by(max(id)).then_order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // multi-column GROUP BY, non-grouped column in order_by, default select
    let _ = users.group_by((name, hair_color)).order_by(id);
    //~^ ERROR: IsContainedInGroupBy

    // -------------------------------------------------------------------------
    // With GROUP BY + LEFT JOIN — non-grouped right-side column in ORDER BY
    // -------------------------------------------------------------------------

    // non-grouped right-side column in order_by, default select
    let _ = parent::table
        .left_join(child::table)
        .group_by(parent::id)
        .order_by(child::value);
    //~^ ERROR: IsContainedInGroupBy

    // non-grouped right-side column in then_order_by after valid grouped order_by, default select
    let _ = parent::table
        .left_join(child::table)
        .group_by(parent::id)
        .order_by(parent::id)
        .then_order_by(child::value);
    //~^ ERROR: IsContainedInGroupBy

    // also check existing order clause
    let _ = parent::table
        .left_join(child::table)
        .order_by(child::value)
        .group_by(parent::id)
        .get_result(conn);
    //~^ ERROR: the trait bound `SkipSelectableExpressionBoundCheckWrapper<...>: ValidGrouping<...>` is not satisfied

    // also check existing order clause
    let _ = parent::table
        .left_join(child::table)
        .order_by(child::value)
        .group_by(parent::id)
        .then_order_by(parent::id);
    //~^ ERROR: the trait bound `SelectStatement<..., ..., ..., ..., ..., ..., ...>: ThenOrderDsl<_>` is not satisfied
}
