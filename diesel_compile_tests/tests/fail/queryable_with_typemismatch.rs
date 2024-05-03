extern crate diesel;

use diesel::prelude::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Queryable)]
struct User {
    id: String,
    name: i32,
}

fn main() {
    let mut conn = PgConnection::establish("...").unwrap();

    users::table.load::<User>(&mut conn).unwrap();
}
