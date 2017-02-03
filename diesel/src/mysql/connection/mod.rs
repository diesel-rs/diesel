mod raw;
mod url;

use connection::{Connection, SimpleConnection};
use query_builder::*;
use query_source::Queryable;
use result::*;
use self::raw::RawConnection;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use types::HasSqlType;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct MysqlConnection {
    _raw_connection: RawConnection,
}

impl SimpleConnection for MysqlConnection {
    fn batch_execute(&self, _query: &str) -> QueryResult<()> {
        unimplemented!()
    }
}

impl Connection for MysqlConnection {
    type Backend = Mysql;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        let raw_connection = RawConnection::new();
        let connection_options = try!(ConnectionOptions::parse(database_url));
        try!(raw_connection.connect(connection_options));
        Ok(MysqlConnection {
            _raw_connection: raw_connection,
        })
    }

    fn execute(&self, _query: &str) -> QueryResult<usize> {
        unimplemented!()
    }

    fn query_all<T, U>(&self, _source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        unimplemented!()
    }

    fn silence_notices<F: FnOnce() -> T, T>(&self, _f: F) -> T {
        unimplemented!()
    }

    fn execute_returning_count<T>(&self, _source: &T) -> QueryResult<usize> {
        unimplemented!()
    }

    fn begin_transaction(&self) -> QueryResult<()> {
        unimplemented!()
    }

    fn rollback_transaction(&self) -> QueryResult<()> {
        unimplemented!()
    }

    fn commit_transaction(&self) -> QueryResult<()> {
        unimplemented!()
    }

    fn get_transaction_depth(&self) -> i32 {
        unimplemented!()
    }

    fn setup_helper_functions(&self) {
        unimplemented!()
    }
}
