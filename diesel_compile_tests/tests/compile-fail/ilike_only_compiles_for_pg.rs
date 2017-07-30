#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[table_name="users"]
struct User {
    id: i32,
    name: String,
}

fn main() {
    let connection = SqliteConnection::establish("").unwrap();
    users::table.filter(users::name.ilike("%hey%")).execute(&connection);
    //~^ ERROR E0271

    let connection = MysqlConnection::establish("").unwrap();
    users::table.filter(users::name.ilike("%hey%")).execute(&connection);
    //~^ ERROR E0271
}
