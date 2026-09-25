extern crate diesel;

use diesel::dsl::{count_star, exists, max, sum};
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
        amount -> Integer,
    }
}

table! {
    toys {
        id -> Integer,
        child_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(parents, children, toys);
allow_columns_to_appear_in_same_group_by_clause!(parents::id, parents::name);

fn main() {
    let mut conn = PgConnection::establish("").unwrap();

    // cases that should compile

    // An uncorrelated subselect is constant for every group
    let _ = parents::table
        .select((
            count_star(),
            children::table.select(max(children::amount)).single_value(),
        ))
        .load::<(i64, Option<i32>)>(&mut conn);
    // A correlated subselect is fine in a query without aggregates
    let _ = parents::table
        .select((
            parents::id,
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .single_value(),
        ))
        .load::<(i32, Option<i64>)>(&mut conn);
    // A correlated subselect may reference columns of the outer `GROUP BY` clause
    let _ = parents::table
        .group_by(parents::id)
        .select((
            parents::id,
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .single_value(),
        ))
        .load::<(i32, Option<i64>)>(&mut conn);
    // Nested subselects may reference any enclosing query
    let _ = parents::table
        .group_by(parents::id)
        .select((
            count_star(),
            exists(
                children::table.filter(children::parent_id.eq(parents::id)).filter(exists(
                    toys::table
                        .filter(toys::child_id.eq(children::id))
                        .filter(toys::id.eq(parents::id)),
                )),
            ),
        ))
        .load::<(i64, bool)>(&mut conn);
    // Aliased outer columns follow the same rules
    let p = alias!(parents as p);
    let _ = p
        .group_by(p.field(parents::id))
        .select((
            p.field(parents::id),
            exists(children::table.filter(children::parent_id.eq(p.field(parents::id)))),
        ))
        .load::<(i32, bool)>(&mut conn);
    // A boxed uncorrelated subselect is constant for every group
    let _ = parents::table
        .select((
            count_star(),
            children::table
                .select(max(children::amount))
                .into_boxed::<pg::Pg>()
                .single_value(),
        ))
        .load::<(i64, Option<i32>)>(&mut conn);

    // cases that should fail to compile

    let _ = parents::table
        .select((
            count_star(),
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .single_value(),
        ))
        //~^^^^^^^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
        .load::<(i64, Option<i64>)>(&mut conn);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    let _ = parents::table
        .group_by(parents::name)
        .select((
            //~^ ERROR: the trait bound `id: ValidGrouping<SubselectGroupBy<name, ...>>` is not satisfied
            parents::name,
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .single_value(),
        ))
        .load::<(String, Option<i64>)>(&mut conn);

    let _ = parents::table
        .group_by(parents::name)
        .select((
            //~^ ERROR: the trait bound `id: ValidGrouping<SubselectGroupBy<..., ...>>` is not satisfied
            parents::name,
            exists(
                children::table.filter(children::parent_id.eq(1)).filter(exists(
                    toys::table.filter(toys::child_id.eq(parents::id)),
                )),
            ),
        ))
        .load::<(String, bool)>(&mut conn);

    let _ = p
        .select((
            count_star(),
            exists(children::table.filter(children::parent_id.eq(p.field(parents::id)))),
        ))
        //~^^^^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL
        .load::<(i64, bool)>(&mut conn);
    //~^ ERROR: mixing aggregate and not aggregate expressions is not allowed in SQL

    // A boxed subselect cannot reference the outer query

    let _ = parents::table
        .select((
            count_star(),
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .into_boxed::<pg::Pg>()
                //~^ ERROR: cannot box `SelectStatement<FromClause<table>, ..., ..., ...>` for backend `Pg`
                .single_value(),
        ))
        .load::<(i64, Option<i64>)>(&mut conn);

    let _ = parents::table
        .select((
            count_star(),
            children::table
                .select(sum(children::amount))
                .into_boxed::<pg::Pg>()
                .filter(children::parent_id.eq(parents::id))
                //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
                .single_value(),
        ))
        .load::<(i64, Option<i64>)>(&mut conn);

    let _ = parents::table
        .select((
            count_star(),
            children::table
                .filter(children::parent_id.eq(parents::id))
                .select(sum(children::amount))
                .into_boxed_clone::<pg::Pg>()
                //~^ ERROR: cannot box `SelectStatement<FromClause<table>, ..., ..., ...>` for backend `Pg`
                .single_value(),
        ))
        .load::<(i64, Option<i64>)>(&mut conn);
}
