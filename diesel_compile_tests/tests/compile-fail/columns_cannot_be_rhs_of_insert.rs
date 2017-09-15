#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> Text,
        hair_color -> Text,
    }
}

fn main() {
    use users::dsl::*;
    let conn = PgConnection::establish("").unwrap();

    insert(&name.eq(hair_color))
        .into(users)
        .execute(&conn)
        //~^ ERROR E0599
        .unwrap();
}
