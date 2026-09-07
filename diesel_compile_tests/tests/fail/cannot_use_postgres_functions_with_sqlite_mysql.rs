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
    //~^ ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`
    //~| ERROR: type mismatch resolving `<SqliteConnection as Connection>::Backend == Pg`
    let _ = query.execute(mysql_connection);
    //~^ ERROR: type mismatch resolving `<MysqlLikeConnection<Mysql> as Connection>::Backend == Pg`
    //~| ERROR: type mismatch resolving `<MysqlLikeConnection<Mysql> as Connection>::Backend == Pg`
}
