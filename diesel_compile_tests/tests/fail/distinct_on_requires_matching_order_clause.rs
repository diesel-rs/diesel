extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

fn main() {

    // verify that we could use distinct on without order clause
    let _ = users::table.distinct_on(users::name);

    // verify that we could use distinct on with an order clause containing the same column
    let _ = users::table.order_by(users::name).distinct_on(users::name);

    // verify that we could use distinct on with an order clause that contains also a different column
    let _ = users::table.order_by((users::name, users::id)).distinct_on(users::name);

    // verify that we could use multiple columns for both order by and distinct on
    let _ = users::table.order_by((users::name, users::id)).distinct_on((users::name, users::id));

    // verify that we could use multiple columns for both order by and distinct on and distinct on has more columns than order by
    let _ = users::table.order_by((users::name, users::id)).distinct_on((users::name, users::id, users::hair_color));

    // verify that we could use multiple columns for both order by and distinct on and distinct on has less columns than order by
    let _ = users::table.order_by((users::name, users::id, users::hair_color)).distinct_on((users::name, users::id));

    // verify that we could use distinct on with a select expression and an order clause that contains a different column
    let _ = users::table
        .distinct_on(users::id)
        .select(users::id)
        .order_by((users::id, users::name));

    // verify that this works also with `then_order_by`
    let _ = users::table
        .order_by(users::name)
        .then_order_by(users::id)
        .distinct_on(users::name);

    // same as above, with asc/desc
    let _ = users::table
        .order_by(users::name.asc())
        .distinct_on(users::name)
        .into_boxed();
    let _ = users::table
        .order_by((users::name.asc(), users::id.desc()))
        .distinct_on(users::name)
        .into_boxed();
    let _ = users::table
        .order_by((users::name.asc(), users::id.desc()))
        .distinct_on((users::name, users::id))
        .into_boxed();
    let _ = users::table
        .order_by((users::name.asc(), users::id.desc()))
        .distinct_on((users::name, users::id, users::hair_color))
        .into_boxed();
    let _ = users::table
        .order_by((users::name.asc(), users::id.desc(), users::hair_color))
        .distinct_on((users::name, users::id))
        .into_boxed();
    let _ = users::table
        .order_by(users::name.asc())
        .then_order_by(users::id)
        .distinct_on(users::name)
        .into_boxed();

    // order by and distinct on with sql literal
    let _ = users::table
        .order(dsl::sql::<sql_types::Bool>("name"))
        .distinct_on(dsl::sql::<sql_types::Bool>("name"));

    // verify that this all works with boxed queries
    let _ = users::table.distinct_on(users::name).into_boxed();
    let _ = users::table
        .order_by(users::name)
        .distinct_on(users::name)
        .into_boxed();
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on(users::name)
        .into_boxed();
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on((users::name, users::id))
        .into_boxed();
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on((users::name, users::id, users::hair_color))
        .into_boxed();
    let _ = users::table
        .order_by((users::name, users::id, users::hair_color))
        .distinct_on((users::name, users::id))
        .into_boxed();
    let _ = users::table
        .order_by(users::name)
        .then_order_by(users::id)
        .distinct_on(users::name)
        .into_boxed();
    // order by and distinct on with sql literal
    let _ = users::table
        .order(dsl::sql::<sql_types::Bool>("name"))
        .distinct_on(dsl::sql::<sql_types::Bool>("name"))
        .into_boxed();

    // compile fail section
    //
    // we do not allow queries with order clauses that does not contain the distinct value
    let _ = users::table.order_by(users::id).distinct_on(users::name);

    // we do not allow queries where the distinct on expression is not the first expression
    // in our order clause
    let _ = users::table.order_by((users::id, users::name)).distinct_on(users::name);

    // we cannot workaround that with `then_order_by`
    let _ = users::table
        .order_by(users::id)
        .then_order_by(users::name)
        .distinct_on(users::name);

    // it's not possible to set an invalid order clause after we set
    // the distinct on clause
    let _ = users::table.distinct_on(users::name).order_by(users::id);

    // we cannot box invalid queries
    let _ = users::table.order_by(users::id).distinct_on(users::name).into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for multiple order by and one distinct on
    let _ = users::table
        .order_by((users::id, users::name))
        .distinct_on(users::name)
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for multiple order by and distinct on
    let _ = users::table
        .order_by((users::id, users::name))
        .distinct_on((users::name, users::id))
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for one order by and multiple distinct on
    let _ = users::table
        .order_by(users::id)
        .distinct_on((users::name, users::id))
        .into_boxed();

    // we cannot workaround that with `then_order_by`
    let _ = users::table
        .order_by(users::id)
        .then_order_by(users::name)
        .distinct_on(users::name)
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // the distinct on clause
    let _ = users::table
        .distinct_on(users::name)
        .order_by(users::id)
        .into_boxed();
}
