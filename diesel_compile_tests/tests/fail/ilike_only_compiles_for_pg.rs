extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
}

fn main() {
    let mut connection = SqliteConnection::establish("").unwrap();
    users::table.filter(users::name.ilike("%hey%")).execute(&mut connection);

    let mut connection = MysqlConnection::establish("").unwrap();
    users::table.filter(users::name.ilike("%hey%")).execute(&mut connection);
}
