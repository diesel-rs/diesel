#[macro_use] extern crate diesel;

use diesel::*;
use diesel::pg::Pg;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
}

fn main() {
    let conn = PgConnection::establish("").unwrap();

    let _ = LoadDsl::load::<User>(
    //~^ ERROR type mismatch resolving `<users::table as diesel::query_source::AppearsInFromClause<posts::table>>::Count == diesel::query_source::Once`
        users::table.filter(posts::id.eq(1)),
        &conn,
    );

    let _ = users::table
        .into_boxed::<Pg>()
        .filter(posts::id.eq(1));
        //~^ ERROR AppearsInFromClause

    let _ = users::table.filter(posts::id.eq(1))
        .into_boxed::<Pg>();
        //~^ ERROR BoxedDsl
        // FIXME: It'd be great if this mentioned `AppearsInFromClause` instead...

    let _ = LoadDsl::load::<User>(
    //~^ ERROR AppearsInFromClause
        users::table.filter(users::name.eq(posts::title)),
        &conn,
    );

    let _ = users::table.into_boxed::<Pg>()
        .filter(users::name.eq(posts::title));
        //~^ ERROR AppearsInFromClause

    let _ = users::table
        .filter(users::name.eq(posts::title))
        .into_boxed::<Pg>();
        //~^ ERROR BoxedDsl
        // FIXME: It'd be great if this mentioned `AppearsInFromClause` instead...
}
