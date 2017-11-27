#[macro_use]
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
    let sqlite_connection = SqliteConnection::establish(":memory:").unwrap();

    users.distinct_on(name).get_results(&sqlite_connection);
    //~^ ERROR Backend == diesel::pg::Pg

    let mysql_connection = MysqlConnection::establish("mysql://foo").unwrap();

    users.distinct_on(name).get_results(&mysql_connection);
    //~^ ERROR Backend == diesel::pg::Pg
}
