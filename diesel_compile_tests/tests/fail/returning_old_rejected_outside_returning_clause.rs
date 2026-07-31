extern crate diesel;

use diesel::pg::returning::old;
use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let mut connection = PgConnection::establish("").unwrap();

    // `old(col)` is only meaningful inside a RETURNING clause.
    // Using it in a regular SELECT is rejected at compile time.
    users
        .select(old(name))
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `users::table`
        .load::<String>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `users::table`
        .unwrap();

    // `old(col).nullable()` is also rejected outside RETURNING.
    users
        .select(old(name).nullable())
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<OldIdent>>::Count == Once`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<...>>::Count == Once`
        .load::<Option<String>>(&mut connection)
        //~^ ERROR: cannot select `returning::old_impl::Old<columns::name>` from `users::table`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<OldIdent>>::Count == Once`
        //~| ERROR: type mismatch resolving `<table as AppearsInFromClause<...>>::Count == Once`
        .unwrap();
}
