extern crate diesel;

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

    let connection = MysqlConnection::establish("").unwrap();
    users::table.filter(users::name.ilike("%hey%")).execute(&connection);
}
