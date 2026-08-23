extern crate diesel;

use diesel::prelude::*;
use diesel::{MariadbConnection, MysqlConnection};

table! {
    users (id) {
        id -> Integer,
    }
}

fn main() {
    let mysql_connection = &mut MysqlConnection::establish("").unwrap();
    users::table
        .drop_table()
        .cascade()
        .execute(mysql_connection);
    //~^ ERROR: `DROP TABLE ... CASCADE` has no cascade semantics for the `Mysql` backend

    let mariadb_connection = &mut MariadbConnection::establish("").unwrap();
    users::table
        .drop_table()
        .cascade()
        .execute(mariadb_connection);
    //~^ ERROR: `DROP TABLE ... CASCADE` has no cascade semantics for the `Mariadb` backend
}
