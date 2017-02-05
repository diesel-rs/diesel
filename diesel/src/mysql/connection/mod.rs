mod raw;
mod url;

use connection::{Connection, SimpleConnection, AnsiTransactionManager};
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
    transaction_manager: AnsiTransactionManager,
}

impl SimpleConnection for MysqlConnection {
    fn batch_execute(&self, _query: &str) -> QueryResult<()> {
        unimplemented!()
    }
}

impl Connection for MysqlConnection {
    type Backend = Mysql;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        let raw_connection = RawConnection::new();
        let connection_options = try!(ConnectionOptions::parse(database_url));
        try!(raw_connection.connect(&connection_options));
        Ok(MysqlConnection {
            _raw_connection: raw_connection,
            transaction_manager: AnsiTransactionManager::new(),
        })
    }

    #[doc(hidden)]
    fn execute(&self, _query: &str) -> QueryResult<usize> {
        unimplemented!()
    }

    #[doc(hidden)]
    fn query_all<T, U>(&self, _source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        unimplemented!()
    }

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, _f: F) -> T {
        unimplemented!()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, _source: &T) -> QueryResult<usize> {
        unimplemented!()
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }

    #[doc(hidden)]
    fn setup_helper_functions(&self) {
        unimplemented!()
    }
}
