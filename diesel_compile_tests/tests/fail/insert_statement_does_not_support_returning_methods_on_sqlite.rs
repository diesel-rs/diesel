extern crate diesel;

use diesel::deserialize::Queryable;
use diesel::sqlite::SqliteConnection;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

#[derive(Queryable)]
pub struct User {
    id: i32,
    name: String,
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] String);

fn main() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .get_result::<User>(&mut connection);
    //~^ ERROR: `ReturningClause<(columns::id, columns::name)>` is no valid SQL fragment for the `Sqlite` backend

    insert_into(users::table)
        .values(&NewUser("Hello".into()))
        .returning(users::name)
        .get_result::<String>(&mut connection);
    //~^ ERROR: `ReturningClause<columns::name>` is no valid SQL fragment for the `Sqlite` backend
}
