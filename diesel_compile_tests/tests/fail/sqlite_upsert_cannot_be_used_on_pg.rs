extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
}

fn main() {
    let mut connection = PgConnection::establish("").unwrap();
    replace_into(users::table)
        .values(&User { id: 1 })
        .execute(&mut connection);
    //~^ ERROR: `diesel::query_builder::insert_statement::private::Replace` is no valid SQL fragment for the `Pg` backend
}
