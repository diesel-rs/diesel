#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::dsl::count;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    let conn = PgConnection::establish("").unwrap();
    let _ = users::table.group_by(users::name)
        .load::<(i32, String)>(&conn);
    //~^ ERROR ValidGrouping
}
