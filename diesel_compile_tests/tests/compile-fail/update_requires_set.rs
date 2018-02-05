#[macro_use] extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
    }
}

fn main() {
    let conn = SqliteConnection::establish(":memory:").unwrap();
    update(users::table)
        .execute(&conn);
        //~^ ERROR diesel::query_builder::update_statement::SetNotCalled: diesel::query_builder::QueryFragment<_>
}
