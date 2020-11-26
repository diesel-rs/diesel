#[macro_use]
extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
    }
}

joinable!(posts -> users (user_id));

fn main() {
    use diesel::dsl::*;

    let conn = PgConnection::establish("connection-url").unwrap();

    // a boxed query with group by just works
    users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed()
        .load::<String>(&conn);

    // it's fine to change the select clause afterwards as long as it is valid for the
    // given group by clause
    users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed()
        .select(users::name)
        .load::<String>(&conn);

    let mut q = users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed();

    q = q.select(users::name);
    q = q.filter(users::id.eq(42));
    q = q.or_filter(users::id.eq(42));
    q = q.order(users::id.asc());
    q = q.then_order_by(users::id.asc());
    q = q.distinct();
    q = q.limit(42);
    q = q.offset(42);

    // cannot box a query with default select clause + a group by clause
    users::table.group_by(users::name).into_boxed();
    //~^ ERROR BoxedDsl

    users::table
        .group_by(users::name)
        .select(users::id)
        //~^ ERROR IsContainedInGroupBy
        .into_boxed();
    //~^ ERROR BoxedDsl

    users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed()
        .select(users::id)
        //~^ ERROR IsContainedInGroupBy<users::columns::id>
        .load::<i32>(&conn);

    users::table
        .group_by(users::name)
        .select(users::name)
        .into_boxed()
        .inner_join(posts::table)
        //~^ ERROR Table
        //~| mismatched types
        .load::<String>(&conn);

    let mut a = users::table.into_boxed();

    // this is a different type now
    a = users::table.group_by(users::id).into_boxed();
    //~^ ERROR mismatched types

    // you cannot call group by after boxing
    users::table
        .into_boxed()
        .group_by(users::id)
        //~^ ERROR type mismatch
        //~| ERROR Table
        //~| ERROR GroupByDsl
        .select(users::name)
        .load::<String>(&conn);
}
