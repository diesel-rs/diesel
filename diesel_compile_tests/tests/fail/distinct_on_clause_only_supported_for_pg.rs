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
    let mut sqlite_connection = SqliteConnection::establish(":memory:").unwrap();

    users.distinct_on(name).get_results(&mut sqlite_connection);

    let mut mysql_connection = MysqlConnection::establish("mysql://foo").unwrap();

    users.distinct_on(name).get_results(&mut mysql_connection);
}
