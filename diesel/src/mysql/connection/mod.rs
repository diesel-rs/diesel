mod bind;
mod raw;
mod stmt;
mod url;

use self::raw::RawConnection;
use self::stmt::Statement;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::*;

#[allow(missing_debug_implementations, missing_copy_implementations)]
/// A connection to a MySQL database. Connection URLs should be in the form
/// `mysql://[user[:password]@]host/database_name`
pub struct MysqlConnection {
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    statement_cache: StatementCache<Mysql, Statement>,
    current_statement: Option<Statement>,
}

unsafe impl Send for MysqlConnection {}

impl SimpleConnection for MysqlConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.raw_connection
            .enable_multi_statements(|| self.raw_connection.execute(query))
    }
}

impl<'a> IterableConnection<'a, Mysql> for MysqlConnection {
    type Cursor = self::stmt::iterator::StatementIterator<'a>;
    type Row = self::stmt::iterator::MysqlRow<'a>;
}

impl Connection for MysqlConnection {
    type Backend = Mysql;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        use crate::result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::new();
        let connection_options = ConnectionOptions::parse(database_url)?;
        raw_connection.connect(&connection_options)?;
        let mut conn = MysqlConnection {
            raw_connection,
            transaction_state: AnsiTransactionManager::default(),
            statement_cache: StatementCache::new(),
            current_statement: None,
        };
        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&mut self, query: &str) -> QueryResult<usize> {
        self.raw_connection
            .execute(query)
            .map(|_| self.raw_connection.affected_rows())
    }

    #[doc(hidden)]
    fn load<'a, T>(
        &'a mut self,
        source: T,
    ) -> QueryResult<<Self as IterableConnection<'a, Self::Backend>>::Cursor>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        self.with_prepared_query(&source.as_query(), |stmt, current_statement| {
            let mut metadata = Vec::new();
            Mysql::row_metadata(&mut (), &mut metadata);
            let stmt = match stmt {
                MaybeCached::CannotCache(stmt) => {
                    *current_statement = Some(stmt);
                    current_statement
                        .as_mut()
                        .expect("We set it literally above")
                }
                MaybeCached::Cached(stmt) => stmt,
            };

            let results = unsafe { stmt.results(metadata)? };
            Ok(results)
        })
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        self.with_prepared_query(source, |stmt, _| {
            unsafe {
                stmt.execute()?;
            }
            Ok(stmt.affected_rows())
        })
    }

    #[doc(hidden)]
    fn transaction_state(&mut self) -> &mut AnsiTransactionManager {
        &mut self.transaction_state
    }
}

impl MysqlConnection {
    fn with_prepared_query<'a, T: QueryFragment<Mysql> + QueryId, R>(
        &'a mut self,
        source: &'_ T,
        f: impl FnOnce(MaybeCached<'a, Statement>, &'a mut Option<Statement>) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let cache = &mut self.statement_cache;
        let conn = &mut self.raw_connection;

        let mut stmt = cache.cached_statement(source, &[], |sql| conn.prepare(sql))?;
        let mut bind_collector = RawBytesBindCollector::new();
        source.collect_binds(&mut bind_collector, &mut ())?;
        let binds = bind_collector
            .metadata
            .into_iter()
            .zip(bind_collector.binds);
        stmt.bind(binds)?;
        f(stmt, &mut self.current_statement)
    }

    fn set_config_options(&mut self) -> QueryResult<()> {
        self.execute("SET sql_mode=(SELECT CONCAT(@@sql_mode, ',PIPES_AS_CONCAT'))")?;
        self.execute("SET time_zone = '+00:00';")?;
        self.execute("SET character_set_client = 'utf8mb4'")?;
        self.execute("SET character_set_connection = 'utf8mb4'")?;
        self.execute("SET character_set_results = 'utf8mb4'")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenv;

    use super::*;
    use std::env;

    fn connection() -> MysqlConnection {
        dotenv::dotenv().ok();
        let database_url = env::var("MYSQL_UNIT_TEST_DATABASE_URL")
            .or_else(|_| env::var("MYSQL_DATABASE_URL"))
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run unit tests");
        MysqlConnection::establish(&database_url).unwrap()
    }

    #[test]
    fn batch_execute_handles_single_queries_with_results() {
        let connection = &mut connection();
        assert!(connection.batch_execute("SELECT 1").is_ok());
        assert!(connection.batch_execute("SELECT 1").is_ok());
    }

    #[test]
    fn batch_execute_handles_multi_queries_with_results() {
        let connection = &mut connection();
        let query = "SELECT 1; SELECT 2; SELECT 3;";
        assert!(connection.batch_execute(query).is_ok());
        assert!(connection.batch_execute(query).is_ok());
    }

    #[test]
    fn execute_handles_queries_which_return_results() {
        let connection = &mut connection();
        assert!(connection.execute("SELECT 1").is_ok());
        assert!(connection.execute("SELECT 1").is_ok());
    }
}
