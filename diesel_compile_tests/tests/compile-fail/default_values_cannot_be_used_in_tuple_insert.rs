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

    ExecuteDsl::execute(
    //~^ ERROR ExecuteDsl
        insert(&(default_values(), name.eq("Sean"))).into(users),
        &conn,
    );
}
