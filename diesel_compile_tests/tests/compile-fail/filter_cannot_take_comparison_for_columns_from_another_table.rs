#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

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

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
}

fn main() {
    let conn = PgConnection::establish("").unwrap();

    let _ = LoadDsl::load::<User>(
    //~^ ERROR AppearsInFromClause
    //~| ERROR E0277
        users::table.filter(posts::id.eq(1)),
        &conn,
    );

    let _ = users::table
        .into_boxed::<Pg>()
        .filter(posts::id.eq(1));
        //~^ ERROR AppearsInFromClause
        //~| ERROR E0277

    let _ = BoxedDsl::into_boxed::<Pg>(
    //~^ ERROR AppearsInFromClause
    //~| ERROR E0277
        users::table.filter(posts::id.eq(1))
    );

    let _ = LoadDsl::load::<User>(
    //~^ ERROR AppearsInFromClause
    //~| ERROR E0277
        users::table.filter(users::name.eq(posts::title)),
        &conn,
    );

    let _ = users::table.into_boxed::<Pg>()
        .filter(users::name.eq(posts::title));
        //~^ ERROR AppearsInFromClause
        //~| ERROR E0277

    let _ = BoxedDsl::into_boxed::<Pg>(
    //~^ ERROR AppearsInFromClause
    //~| ERROR E0277
        users::table
            .filter(users::name.eq(posts::title))
    );
}
