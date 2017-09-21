#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

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
    replace_into(users::table)
        .values(&User { id: 1 })
        .execute(&connection);
        //~^ ERROR E0277
}
