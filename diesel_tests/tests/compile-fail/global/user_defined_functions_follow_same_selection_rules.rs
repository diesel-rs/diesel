#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::types::*;

table! {
    users {
        id -> Serial,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Serial,
        title -> VarChar,
    }
}

sql_function!(foo, foo_t, (x: Integer) -> Integer);
sql_function!(bar, bar_t, (x: VarChar) -> VarChar);

fn main() {
    use self::users::name;
    use self::posts::title;

    let _ = users::table.filter(name.eq(foo(1)));
    //~^ ERROR type mismatch
    let _ = users::table.filter(name.eq(bar(title)));
    //~^ ERROR E0277
}
