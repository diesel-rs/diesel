extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Nullable<Text>,
    }
}

fn main() {
    let conn = &mut PgConnection::establish("...").unwrap();

    // this query is valid
    diesel::insert_into(users::table)
        .values(&[(users::id.eq(1), users::name.eq("Sean"))])
        .on_conflict(users::id)
        .filter_target(users::hair_color.eq("black"))
        .do_update()
        .set(users::hair_color.eq("red"))
        .execute(conn)
        .unwrap();

    // This is invalid as filter_target is only valid for
    // do update queries
    diesel::insert_into(users::table)
        .values(&[(users::id.eq(1), users::name.eq("Sean"))])
        .on_conflict(users::id)
        .filter_target(users::hair_color.eq("black"))
        .do_nothing()
        .execute(conn)
        .unwrap();
}
