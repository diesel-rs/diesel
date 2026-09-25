extern crate diesel;

use diesel::mariadb::Mariadb;
use diesel::mariadb::returning::old_value;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        user_id -> Integer,
        title -> Text,
    }
}

allow_tables_to_appear_in_same_query!(posts, users);

fn main() {
    let mut conn = MariadbConnection::establish("…").unwrap();

    // any of those are accepted
    let boxed_expr: Box<
        dyn BoxableExpression<users::table, Mariadb, SqlType = diesel::sql_types::Text>,
    > = Box::new(users::name);
    diesel::update(users::table.filter(users::id.eq(42)))
        .set(users::name.eq("Renamed"))
        .returning((
            old_value(users::name),
            users::name,
            posts::table
                .select(posts::title)
                .filter(posts::user_id.eq(users::id))
                .single_value(),
            boxed_expr,
        ))
        .execute(&mut conn)
        .unwrap();

    // this needs to be rejected
    diesel::update(users::table.filter(users::id.eq(42)))
        .set(users::name.eq("Renamed"))
        .returning((posts::table
            //~^ ERROR: the trait bound `OldValue<id>: AppearsOnTable<Join<..., ..., ...>>` is not satisfied
            .select(posts::title)
            .filter(posts::user_id.eq(old_value(users::id)))
            .single_value(),))
        .execute(&mut conn)
        .unwrap();
}
