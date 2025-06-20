extern crate diesel;

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

    delete(users.filter(name.eq("Bill"))).get_result(&mut connection);
    //~^ ERROR: `ReturningClause<(columns::id, columns::name)>` is no valid SQL fragment for the `Sqlite` backend

    delete(users.filter(name.eq("Bill")))
        .returning(name)
        .get_result(&mut connection);
    //~^ ERROR: `ReturningClause<columns::name>` is no valid SQL fragment for the `Sqlite` backend
}
