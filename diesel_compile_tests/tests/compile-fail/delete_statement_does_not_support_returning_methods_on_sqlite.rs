#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::sqlite::SqliteConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;
    let connection = SqliteConnection::establish(":memory:").unwrap();

    delete(users.filter(name.eq("Bill")))
        .get_result(&connection);
    //~^ ERROR SupportsReturningClause

    delete(users.filter(name.eq("Bill")))
        .returning(name)
        .get_result(&connection);
    //~^ ERROR SupportsReturningClause
}
