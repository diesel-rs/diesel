extern crate diesel;

use diesel::connection::{DefaultLoadingMode, LoadConnection};
use diesel::pg::PgRowByRowLoadingMode;
use diesel::prelude::*;
use diesel::sql_query;

fn main() {
    let conn = &mut SqliteConnection::establish("foo").unwrap();
    // For sqlite the returned iterator is coupled to
    // a statement, which is coupled to the connection itself
    // so we cannot have more than one iterator
    // for the same connection
    let row_iter1 = LoadConnection::load(conn, sql_query("bar")).unwrap();
    let row_iter2 = LoadConnection::load(conn, sql_query("bar")).unwrap();
    //~^ ERROR: cannot borrow `*conn` as mutable more than once at a time

    let _ = row_iter1.zip(row_iter2);

    let conn = &mut MysqlConnection::establish("foo").unwrap();
    // The same argument applies to mysql
    let row_iter1 = LoadConnection::load(conn, sql_query("bar")).unwrap();
    let row_iter2 = LoadConnection::load(conn, sql_query("bar")).unwrap();
    //~^ ERROR: cannot borrow `*conn` as mutable more than once at a time

    let _ = row_iter1.zip(row_iter2);

    let conn = &mut PgConnection::establish("foo").unwrap();
    // It works for PgConnection as the result is not related to the
    // connection in any way
    let row_iter1 = LoadConnection::<DefaultLoadingMode>::load(conn, sql_query("bar")).unwrap();
    let row_iter2 = LoadConnection::<DefaultLoadingMode>::load(conn, sql_query("bar")).unwrap();

    let _ = row_iter1.zip(row_iter2);

    // It does not work for the libpq row by row mode
    let row_iter1 = LoadConnection::<PgRowByRowLoadingMode>::load(conn, sql_query("bar")).unwrap();
    let row_iter2 = LoadConnection::<PgRowByRowLoadingMode>::load(conn, sql_query("bar")).unwrap();
    //~^ ERROR: cannot borrow `*conn` as mutable more than once at a time

    let _ = row_iter1.zip(row_iter2);
}
