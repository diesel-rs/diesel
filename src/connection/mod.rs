extern crate postgres;

use super::query_source::{Queriable, QuerySource};
use super::{Result, ConnectionResult};
use super::types::FromSql;
use self::postgres::{SslMode, Statement};

pub struct Connection {
    internal_connection: postgres::Connection,
}

impl Connection {
    pub fn establish(database_url: &str) -> ConnectionResult<Connection> {
        let pgconn = try!(postgres::Connection::connect(database_url, &SslMode::None));
        Ok(Connection {
            internal_connection: pgconn,
        })
    }

    pub fn execute(&self, query: &str) -> Result<u64> {
        self.internal_connection.execute(query, &[])
            .map_err(|e| e.into())
    }

    pub fn query_one<T, U>(&self, source: &T) -> Result<Option<U>> where
        T: QuerySource,
        U: Queriable<T::SqlType>,
    {
        let stmt = try!(self.prepare_query(source));
        let rows = try!(stmt.query(&[]));
        Ok(rows.into_iter().map(|row| {
            let values = U::Row::from_sql(&row, 0);
            U::build(values)
        }).nth(0))
    }

    pub fn query_all<T, U>(&self, source: &T) -> Result<Cursor<U>> where
        T: QuerySource,
        U: Queriable<T::SqlType>,
    {
        let stmt = try!(self.prepare_query(source));
        let rows = try!(stmt.query(&[]));
        let result: Vec<U> = rows.into_iter().map(|row| {
            let values = U::Row::from_sql(&row, 0);
            U::build(values)
        }).collect();
        Ok(Cursor(result.into_iter()))
    }

    fn prepare_query<T: QuerySource>(&self, source: &T) -> Result<Statement> {
        let query = format!("SELECT {} FROM {}", source.select_clause(), source.from_clause());
        self.internal_connection.prepare(&query).map_err(|e| e.into())
    }
}

pub struct Cursor<T>(::std::vec::IntoIter<T>);

impl<T> Iterator for Cursor<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
}
