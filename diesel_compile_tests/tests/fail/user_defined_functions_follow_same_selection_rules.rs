extern crate diesel;

use diesel::sql_types::*;
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

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
}

sql_function_v2!(fn foo(x: Integer) -> Integer);
sql_function_v2!(fn bar(x: VarChar) -> VarChar);

fn main() {
    use self::posts::title;
    use self::users::name;

    let mut conn = PgConnection::establish("").unwrap();

    let _ = users::table.filter(name.eq(foo(1)));

    let _ = users::table
        .filter(name.eq(bar(title)))
        .load::<User>(&mut conn);
}
