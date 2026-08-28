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
    posts {
        id -> Integer,
        title -> Text,
        user_id -> Integer,
        body -> Text,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);
joinable!(posts -> users(user_id));

fn main() {
    // verify that we could use distinct on without order clause
    let _ = users::table.distinct_on(users::name);

    // verify that we could use distinct on with an order clause containing the same column
    let _ = users::table.order_by(users::name).distinct_on(users::name);

    // verify that we could use distinct on with an order clause that contains also a different column
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on(users::name);

    // verify that we could use multiple columns for both order by and distinct on
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on((users::name, users::id));

    // verify that we could use multiple columns for both order by and distinct on and distinct on has more columns than order by
    let _ = users::table
        .order_by((users::name, users::id))
        .distinct_on((users::name, users::id, users::hair_color));

    // verify that we could use multiple columns for both order by and distinct on and distinct on has less columns than order by
    let _ = users::table
        .order_by((users::name, users::id, users::hair_color))
        .distinct_on((users::name, users::id));

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
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // we do not allow queries where the distinct on expression is not the first expression
    // in our order clause
    let _ = users::table
        .order_by((users::id, users::name))
        .distinct_on(users::name);
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // we cannot workaround that with `then_order_by`
    let _ = users::table
        .order_by(users::id)
        .then_order_by(users::name)
        .distinct_on(users::name);
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // it's not possible to set an invalid order clause after we set
    // the distinct on clause
    let _ = users::table.distinct_on(users::name).order_by(users::id);
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // we cannot box invalid queries
    let _ = users::table
        .order_by(users::id)
        .distinct_on(users::name)
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for multiple order by and one distinct on
    let _ = users::table
        .order_by((users::id, users::name))
        .distinct_on(users::name)
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for multiple order by and distinct on
    let _ = users::table
        .order_by((users::id, users::name))
        .distinct_on((users::name, users::id))
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // for one order by and multiple distinct on
    let _ = users::table
        .order_by(users::id)
        .distinct_on((users::name, users::id))
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // we cannot workaround that with `then_order_by`
    let _ = users::table
        .order_by(users::id)
        .then_order_by(users::name)
        .distinct_on(users::name)
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // it's not possible to set an invalid order clause after we set
    // the distinct on clause
    let _ = users::table
        .distinct_on(users::name)
        .order_by(users::id)
        //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
        .into_boxed();

    // verify that we cannot use `then_order_by` to
    // add a not matching element later
    // verify that we could use multiple columns for both order by and distinct on and distinct on has more columns than order by
    //
    // that works
    let _ = users::table
        .order_by(users::name)
        .distinct_on((users::name, users::id));
    // this should fail
    let _ = users::table
        .order_by(users::name)
        .distinct_on((users::name, users::id))
        .then_order_by(users::hair_color);
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // using joins works, also with more than 5 columns
    users::table.inner_join(posts::table).order_by((
        users::id,
        posts::id,
        users::name,
        posts::title,
        users::hair_color,
        posts::body,
    ));
    // the distinct check continues to work
    users::table
        .inner_join(posts::table)
        .distinct_on(users::id)
        .order_by(posts::id);
    //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause

    // we reject ordering by more than 5 columns
    // (If we change the number, it's fine to update this example)
    users::table
        .inner_join(posts::table)
        .distinct_on(users::id)
        .order_by((
            //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
            users::id,
            posts::id,
            users::hair_color,
            users::name,
            posts::title,
            posts::body,
        ));
    // using then_order_by doesn't allow to workaround that
    users::table
        .inner_join(posts::table)
        .distinct_on((users::id, posts::id))
        .order_by(users::id)
        .then_order_by((
            //~^ ERROR: invalid order of elements in your `DISTINCT ON` clause in relation to your `ORDER BY` clause
            posts::id,
            users::hair_color,
            users::name,
            posts::title,
            posts::body,
            posts::user_id,
        ));

    // working around the limitation with tuples works
    // same example as the plain tuple example above
    // with just an extra tuple
    users::table
        .inner_join(posts::table)
        .distinct_on(users::id)
        .order_by((
            users::id,
            (
                posts::id,
                users::hair_color,
                users::name,
                posts::title,
                posts::body,
            ),
        ));
}
