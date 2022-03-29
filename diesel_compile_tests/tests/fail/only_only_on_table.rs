extern crate diesel;

use diesel::dsl::*;
use diesel::*;

table! {
    foo (id) {
        id -> Int8,
    }
}

fn main() {
    foo::table.select(foo::id).only();
    foo::table.select(foo::id).filter(foo::id.eq(1)).only();
    foo::table.select(foo::id.only());

    // .only() is not supported for SQLite
    let mut conn = SqliteConnection::establish("").unwrap();
    foo::table.only().load(&mut conn).unwrap();

    // .only() is not supported for MySql
    let mut conn = MysqlConnection::establish("").unwrap();
    foo::table.only().load(&mut conn).unwrap();
}
