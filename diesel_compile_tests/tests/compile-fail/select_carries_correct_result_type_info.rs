#[macro_use]
extern crate diesel;

use diesel::*;
use diesel::pg::PgConnection;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

fn main() {
    use self::users::dsl::*;

    let connection = PgConnection::establish("").unwrap();
    let select_id = users.select(id);
    let select_name = users.select(name);

    let ids = select_name.load::<i32>(&connection);
    //~^ ERROR the trait bound `i32: diesel::Queryable<diesel::types::Text, _>` is not satisfied
    let names = select_id.load::<String>(&connection);
    //~^ ERROR the trait bound `std::string::String: diesel::Queryable<diesel::types::Integer, _>` is not satisfied
}
