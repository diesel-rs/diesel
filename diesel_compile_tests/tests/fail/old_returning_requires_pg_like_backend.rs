extern crate diesel;

use diesel::pg::returning::old;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    update(users.filter(id.eq(1)))
        .set(name.eq("Dean"))
        .returning(old(name))
        .get_result::<String>(&mut connection);
    //~^ ERROR: `ReturningClause<Old<name>>` is no valid SQL fragment for the `Sqlite` backend
}
