#[macro_use] extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)]
#[table_name="users"]
struct User {
    id: i32,
}

fn main() {
    let connection = PgConnection::establish("").unwrap();
    insert_or_ignore_into(users::table)
        .values(users::id.eq(1))
        .execute(&connection)
        //~^ ERROR E0277
        .unwrap();
}
