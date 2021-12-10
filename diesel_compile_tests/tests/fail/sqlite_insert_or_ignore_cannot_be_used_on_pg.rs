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
    insert_or_ignore_into(users::table)
        .values(users::id.eq(1))
        .execute(&mut connection)
        .unwrap();
}
