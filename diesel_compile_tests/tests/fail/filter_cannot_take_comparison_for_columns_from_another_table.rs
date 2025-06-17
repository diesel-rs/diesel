extern crate diesel;

use diesel::pg::Pg;
use diesel::*;

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
    let mut conn = PgConnection::establish("").unwrap();

    let _ = users::table.filter(posts::id.eq(1)).load::<User>(&mut conn);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    let _ = users::table.into_boxed::<Pg>().filter(posts::id.eq(1));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    let _ = users::table.filter(posts::id.eq(1)).into_boxed::<Pg>();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ..., ...>: BoxedDsl<'_, ...>` is not satisfied
    // FIXME: It'd be great if this mentioned `AppearsInFromClause` instead...

    let _ = users::table
        .filter(users::name.eq(posts::title))
        .load::<User>(&mut conn);
    //~^ ERROR: AppearsInFromClause

    let _ = users::table
        .into_boxed::<Pg>()
        .filter(users::name.eq(posts::title));
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`

    let _ = users::table
        .filter(users::name.eq(posts::title))
        .into_boxed::<Pg>();
    //~^ ERROR: the trait bound `SelectStatement<FromClause<table>, ..., ..., ...>: BoxedDsl<'_, ...>` is not satisfied
    // FIXME: It'd be great if this mentioned `AppearsInFromClause` instead...
}
