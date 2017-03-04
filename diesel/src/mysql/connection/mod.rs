mod bind;
mod raw;
mod stmt;
mod url;

use connection::*;
use query_builder::*;
use query_builder::bind_collector::RawBytesBindCollector;
use query_source::Queryable;
use result::*;
use self::raw::RawConnection;
use self::stmt::Statement;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use types::HasSqlType;

#[allow(missing_debug_implementations, missing_copy_implementations)]
/// A connection to a MySQL database. Connection URLs should be in the form
/// `mysql://[user[:password]@]host/database_name`
pub struct MysqlConnection {
    raw_connection: RawConnection,
    transaction_manager: AnsiTransactionManager,
    statement_cache: StatementCache<Mysql, Statement>,
}

unsafe impl Send for MysqlConnection {}

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
        use result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::new();
        let connection_options = try!(ConnectionOptions::parse(database_url));
        try!(raw_connection.connect(&connection_options));
        let conn = MysqlConnection {
            raw_connection: raw_connection,
            transaction_manager: AnsiTransactionManager::new(),
            statement_cache: StatementCache::new(),
        };
        try!(conn.set_config_options().map_err(CouldntSetupConfiguration));
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.raw_connection.execute(query)
            .map(|_| self.raw_connection.affected_rows())
    }

    #[doc(hidden)]
    fn query_all<T, U>(&self, source: T) -> QueryResult<Vec<U>> where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        use result::Error::DeserializationError;
        use types::FromSqlRow;

        let mut stmt = try!(self.prepare_query(&source.as_query()));
        let mut metadata = Vec::new();
        Mysql::row_metadata(&mut metadata);
        let results = unsafe { stmt.results(metadata)? };
        results.map(|mut row| {
            U::Row::build_from_row(&mut row)
                .map(U::build)
                .map_err(DeserializationError)
        })
    }

    #[doc(hidden)]
    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        f()
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize> where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let stmt = try!(self.prepare_query(source));
        unsafe { try!(stmt.execute()); }
        Ok(stmt.affected_rows())
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }

    #[doc(hidden)]
    fn setup_helper_functions(&self) {
        // FIXME: We can implement this pretty easily
    }
}

impl MysqlConnection {
    fn prepare_query<T>(&self, source: &T) -> QueryResult<MaybeCached<Statement>> where
        T: QueryFragment<Mysql> + QueryId,
    {
        let mut stmt = self.statement_cache.cached_statement(source, &[], |sql| {
            self.raw_connection.prepare(sql)
        })?;
        let mut bind_collector = RawBytesBindCollector::<Mysql>::new();
        try!(source.collect_binds(&mut bind_collector));
        let metadata = bind_collector.metadata;
        let binds = bind_collector.binds;
        try!(stmt.bind(metadata.into_iter().zip(binds)));
        Ok(stmt)
    }

    fn set_config_options(&self) -> QueryResult<()> {
        try!(self.execute("SET sql_mode=(SELECT CONCAT(@@sql_mode, ',PIPES_AS_CONCAT'))"));
        Ok(())
    }
}
