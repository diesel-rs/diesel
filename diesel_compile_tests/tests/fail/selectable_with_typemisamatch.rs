extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
struct User {
    id: String,
    name: i32,
}


fn main() {
    let mut conn = PgConnection::establish("...").unwrap();

    users::table.select(User::as_select()).load(&mut conn).unwrap();


}
