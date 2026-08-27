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

    let pg_connection = &mut PgConnection::establish("…").unwrap();
    let mysql_connection = &mut MysqlConnection::establish("…").unwrap();

    let query = users::table.select(json(users::name));

    let _ = query.execute(pg_connection);
    //~^ ERROR: type mismatch resolving `<PgConnection as Connection>::Backend == Sqlite`
    //~| ERROR: type mismatch resolving `<PgConnection as Connection>::Backend == Sqlite`
    let _ = query.execute(mysql_connection);
    //~^ ERROR: type mismatch resolving `<MysqlLikeConnection<Mysql> as Connection>::Backend == Sqlite`
    //~| ERROR: type mismatch resolving `<MysqlLikeConnection<Mysql> as Connection>::Backend == Sqlite`
}
