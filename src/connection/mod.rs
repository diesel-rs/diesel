extern crate postgres;

use super::query_source::{Queriable, QuerySource};
use super::{Result, ConnectionResult};
use super::types::FromSql;
use self::postgres::SslMode;

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

    pub fn query_all<T, U>(&self, source: &T) -> Result<Vec<U>> where
        T: QuerySource,
        U: Queriable<T>,
    {
        let query = format!("SELECT {} FROM {}", source.select_clause(), source.from_clause());
        let stmt = try!(self.internal_connection.prepare(&query));
        let rows = try!(stmt.query(&[]));
        Ok(rows.into_iter().map(|row| {
            let values = U::Row::from_sql(&row, 0);
            U::build(values)
        }).collect())
    }
}
