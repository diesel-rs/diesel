extern crate diesel;

use diesel::prelude::*;
use diesel::sql_query;

fn main() {
    let conn = &mut SqliteConnection::establish("foo").unwrap();
    // For sqlite the returned iterator is coupled to
    // a statement, which is coupled to the connection itself
    // so we cannot have more than one iterator
    // for the same connection
    let row_iter1 = conn.load(&sql_query("bar")).unwrap();
    let row_iter2 = conn.load(&sql_query("bar")).unwrap();

    let _ = row_iter1.zip(row_iter2);

    let conn = &mut MysqlConnection::establish("foo").unwrap();
    // The same argument applies to mysql
    let row_iter1 = conn.load(&sql_query("bar")).unwrap();
    let row_iter2 = conn.load(&sql_query("bar")).unwrap();

    let _ = row_iter1.zip(row_iter2);

    let conn = &mut PgConnection::establish("foo").unwrap();
    // It works for PgConnection as the result is not related to the
    // connection in any way
    let row_iter1 = conn.load(&sql_query("bar")).unwrap();
    let row_iter2 = conn.load(&sql_query("bar")).unwrap();

    let _ = row_iter1.zip(row_iter2);

}
