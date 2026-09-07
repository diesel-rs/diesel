extern crate diesel;

use diesel::prelude::*;
use diesel::SqliteConnection;

table! {
    users (id) {
        id -> Integer,
    }
}

fn main() {
    let connection = &mut SqliteConnection::establish(":memory:").unwrap();

    users::table.drop_table().cascade().execute(connection);
    //~^ ERROR: `DROP TABLE ... CASCADE` has no cascade semantics for the `Sqlite` backend
}
