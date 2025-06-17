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

    let mut connection = PgConnection::establish("").unwrap();
    let select_id = users.select(id);
    let select_name = users.select(name);

    let ids = select_name.load::<i32>(&mut connection);
    //~^ ERROR: cannot deserialize a value of the database type `diesel::sql_types::Text` as `i32`
    let names = select_id.load::<String>(&mut connection);
    //~^ ERROR: cannot deserialize a value of the database type `diesel::sql_types::Integer` as `std::string::String`
}
