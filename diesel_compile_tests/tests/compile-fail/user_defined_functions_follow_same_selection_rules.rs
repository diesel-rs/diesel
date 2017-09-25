#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

use diesel::*;
use diesel::types::*;

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

sql_function!(foo, foo_t, (x: Integer) -> Integer);
sql_function!(bar, bar_t, (x: VarChar) -> VarChar);

fn main() {
    use self::users::name;
    use self::posts::title;

    let conn = PgConnection::establish("").unwrap();

    let _ = users::table.filter(name.eq(foo(1)));
    //~^ ERROR type mismatch

    let _ = LoadDsl::load::<User>(
    //~^ ERROR E0277
    //~| ERROR E0271
        users::table.filter(name.eq(bar(title))),
        &conn,
    );
}
