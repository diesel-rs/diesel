extern crate diesel;

use diesel::*;
use diesel::sql_types::*;

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

sql_function!(fn foo(x: Integer) -> Integer);
sql_function!(fn bar(x: VarChar) -> VarChar);

fn main() {
    use self::users::name;
    use self::posts::title;

    let mut conn = PgConnection::establish("").unwrap();

    let _ = users::table.filter(name.eq(foo(1)));

    let _ = users::table.filter(name.eq(bar(title)))
        .load::<User>(&mut conn);
}
