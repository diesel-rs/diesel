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
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User {
    id: String,
    name: i32,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserCorrect {
    id: i32,
    name: String,
}

fn main() {
    let mut conn = PgConnection::establish("...").unwrap();

    users::table
        .select(User::as_select())
        .load(&mut conn)
        .unwrap();
    users::table
        .select(UserCorrect::as_select())
        .load(&mut conn)
        .unwrap();
}
