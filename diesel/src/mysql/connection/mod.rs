mod bind;
mod raw;
mod stmt;
mod url;

use connection::{Connection, SimpleConnection, AnsiTransactionManager};
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::Queryable;
use result::*;
use self::raw::RawConnection;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use super::query_builder::MysqlQueryBuilder;
use types::HasSqlType;

#[allow(missing_debug_implementations, missing_copy_implementations)]
pub struct MysqlConnection {
    raw_connection: RawConnection,
    transaction_manager: AnsiTransactionManager,
}

impl SimpleConnection for MysqlConnection {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        self.raw_connection.enable_multi_statements(|| {
            self.raw_connection.execute(query)
        })
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
            raw_connection: raw_connection,
            transaction_manager: AnsiTransactionManager::new(),
        })
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.raw_connection.execute(query)
            .map(|_| self.raw_connection.affected_rows())
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
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let mut query_builder = MysqlQueryBuilder::new();
        try!(source.to_sql(&mut query_builder).map_err(Error::QueryBuilderError));
        let mut bind_collector = RawBytesBindCollector::<Mysql>::new();
        try!(source.collect_binds(&mut bind_collector));
        let mut stmt = try!(self.raw_connection.prepare(&query_builder.sql));
        try!(stmt.bind(bind_collector.binds));
        try!(stmt.execute());
        Ok(stmt.affected_rows())
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
