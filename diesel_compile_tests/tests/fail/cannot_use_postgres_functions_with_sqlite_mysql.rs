extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> Text,
    }
}

fn main() {
    use diesel::dsl::*;

    let sqlite_connection = &mut SqliteConnection::establish("…").unwrap();
    let mysql_connection = &mut MysqlConnection::establish("…").unwrap();

    let query = users::table.select(to_json(users::name));

    let _ = query.execute(sqlite_connection);
    //~^ ERROR: `to_json<Text, name>` is no valid SQL fragment for the `Sqlite` backend
    let _ = query.execute(mysql_connection);
    //~^ ERROR: `to_json<Text, name>` is no valid SQL fragment for the `Mysql` backend
}
