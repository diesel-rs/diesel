mod bind;
mod raw;
mod stmt;
mod url;

use self::raw::RawConnection;
use self::stmt::Statement;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use super::bind_collector::MysqlBindCollector;
use crate::connection::*;
use crate::deserialize::{Queryable, QueryableByName};
use crate::query_builder::*;
use crate::result::*;
use crate::sql_types::HasSqlType;

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
        self.raw_connection
            .enable_multi_statements(|| self.raw_connection.execute(query))
    }
}

impl Connection for MysqlConnection {
    type Backend = Mysql;
    type TransactionManager = AnsiTransactionManager;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        use crate::result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::new();
        let connection_options = ConnectionOptions::parse(database_url)?;
        raw_connection.connect(&connection_options)?;
        let conn = MysqlConnection {
            raw_connection: raw_connection,
            transaction_manager: AnsiTransactionManager::new(),
            statement_cache: StatementCache::new(),
        };
        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    #[doc(hidden)]
    fn execute(&self, query: &str) -> QueryResult<usize> {
        self.raw_connection
            .execute(query)
            .map(|_| self.raw_connection.affected_rows())
    }

    #[doc(hidden)]
    fn query_by_index<T, U>(&self, source: T) -> QueryResult<Vec<U>>
    where
        T: AsQuery,
        T::Query: QueryFragment<Self::Backend> + QueryId,
        Self::Backend: HasSqlType<T::SqlType>,
        U: Queryable<T::SqlType, Self::Backend>,
    {
        use crate::deserialize::FromSqlRow;
        use crate::result::Error::DeserializationError;

        let mut stmt = self.prepare_query(&source.as_query())?;
        let mut metadata = Vec::new();
        Mysql::mysql_row_metadata(&mut metadata, &());
        let results = unsafe { stmt.results(metadata)? };
        results.map(|mut row| {
            U::Row::build_from_row(&mut row)
                .map(U::build)
                .map_err(DeserializationError)
        })
    }

    #[doc(hidden)]
    fn query_by_name<T, U>(&self, source: &T) -> QueryResult<Vec<U>>
    where
        T: QueryFragment<Self::Backend> + QueryId,
        U: QueryableByName<Self::Backend>,
    {
        use crate::result::Error::DeserializationError;

        let mut stmt = self.prepare_query(source)?;
        let results = unsafe { stmt.named_results()? };
        results.map(|row| U::build(&row).map_err(DeserializationError))
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let stmt = self.prepare_query(source)?;
        unsafe {
            stmt.execute()?;
        }
        Ok(stmt.affected_rows())
    }

    #[doc(hidden)]
    fn transaction_manager(&self) -> &Self::TransactionManager {
        &self.transaction_manager
    }
}

impl MysqlConnection {
    fn prepare_query<T>(&self, source: &T) -> QueryResult<MaybeCached<Statement>>
    where
        T: QueryFragment<Mysql> + QueryId,
    {
        let mut stmt = self
            .statement_cache
            .cached_statement(source, &[], |sql| self.raw_connection.prepare(sql))?;
        let mut bind_collector = MysqlBindCollector::new();
        source.collect_binds(&mut bind_collector, &())?;
        stmt.bind(bind_collector.binds)?;
        Ok(stmt)
    }

    fn set_config_options(&self) -> QueryResult<()> {
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
        let _ = dotenv::dotenv();
        let database_url = env::var("MYSQL_UNIT_TEST_DATABASE_URL")
            .or_else(|_| env::var("MYSQL_DATABASE_URL"))
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("DATABASE_URL must be set in order to run unit tests");
        MysqlConnection::establish(&database_url).unwrap()
    }

    #[test]
    fn batch_execute_handles_single_queries_with_results() {
        let connection = connection();
        assert!(connection.batch_execute("SELECT 1").is_ok());
        assert!(connection.batch_execute("SELECT 1").is_ok());
    }

    #[test]
    fn batch_execute_handles_multi_queries_with_results() {
        let connection = connection();
        let query = "SELECT 1; SELECT 2; SELECT 3;";
        assert!(connection.batch_execute(query).is_ok());
        assert!(connection.batch_execute(query).is_ok());
    }

    #[test]
    fn execute_handles_queries_which_return_results() {
        let connection = connection();
        assert!(connection.execute("SELECT 1").is_ok());
        assert!(connection.execute("SELECT 1").is_ok());
    }
}
