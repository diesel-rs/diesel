mod bind;
mod raw;
mod stmt;
mod url;

use self::raw::RawConnection;
use self::stmt::iterator::StatementIterator;
use self::stmt::Statement;
use self::url::ConnectionOptions;
use super::backend::Mysql;
use crate::connection::commit_error_processor::{
    default_process_commit_error, CommitErrorOutcome, CommitErrorProcessor,
};
use crate::connection::statement_cache::{MaybeCached, StatementCache};
use crate::connection::*;
use crate::expression::QueryMetadata;
use crate::query_builder::bind_collector::RawBytesBindCollector;
use crate::query_builder::*;
use crate::result::*;
use crate::RunQueryDsl;

#[cfg(feature = "mysql")]
#[allow(missing_debug_implementations, missing_copy_implementations)]
/// A connection to a MySQL database. Connection URLs should be in the form
/// `mysql://[user[:password]@]host/database_name`
pub struct MysqlConnection {
    raw_connection: RawConnection,
    transaction_state: AnsiTransactionManager,
    statement_cache: StatementCache<Mysql, Statement>,
}

unsafe impl Send for MysqlConnection {}

impl SimpleConnection for MysqlConnection {
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        self.raw_connection
            .enable_multi_statements(|| self.raw_connection.execute(query))
    }
}

impl<'conn, 'query> ConnectionGatWorkaround<'conn, 'query, Mysql> for MysqlConnection {
    type Cursor = self::stmt::iterator::StatementIterator<'conn>;
    type Row = self::stmt::iterator::MysqlRow;
}

impl CommitErrorProcessor for MysqlConnection {
    fn process_commit_error(&self, error: Error) -> CommitErrorOutcome {
        let state = match self.transaction_state.status {
            TransactionManagerStatus::InError => {
                return CommitErrorOutcome::Throw(Error::BrokenTransaction)
            }
            TransactionManagerStatus::Valid(ref v) => v,
        };
        default_process_commit_error(state, error)
    }
}

impl Connection for MysqlConnection {
    type Backend = Mysql;
    type TransactionManager = AnsiTransactionManager;

    /// Establishes a new connection to the MySQL database
    /// `database_url` may be enhanced by GET parameters
    /// `mysql://[user[:password]@]host/database_name[?unix_socket=socket-path&ssl_mode=SSL_MODE*&ssl_ca=/etc/ssl/certs/ca-certificates.crt]`
    ///
    /// * `unix_socket` expects the path to the unix socket
    /// * `ssl_ca` accepts a path to the system's certificate roots
    /// * `ssl_mode` expects a value defined for MySQL client command option `--ssl-mode`
    /// See <https://dev.mysql.com/doc/refman/5.7/en/connection-options.html#option_general_ssl-mode>
    fn establish(database_url: &str) -> ConnectionResult<Self> {
        use crate::result::ConnectionError::CouldntSetupConfiguration;

        let raw_connection = RawConnection::new();
        let connection_options = ConnectionOptions::parse(database_url)?;
        raw_connection.connect(&connection_options)?;
        let mut conn = MysqlConnection {
            raw_connection,
            transaction_state: AnsiTransactionManager::default(),
            statement_cache: StatementCache::new(),
        };
        conn.set_config_options()
            .map_err(CouldntSetupConfiguration)?;
        Ok(conn)
    }

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<LoadRowIter<'conn, 'query, Self, Self::Backend>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        let stmt = self.prepared_query(&source)?;

        let mut metadata = Vec::new();
        Mysql::row_metadata(&mut (), &mut metadata);

        StatementIterator::from_stmt(stmt, &metadata)
    }

    #[doc(hidden)]
    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        let err = {
            let stmt = self.prepared_query(source)?;
            let res = unsafe { stmt.execute() };
            match res {
                Ok(stmt_use) => return Ok(stmt_use.affected_rows()),
                Err(e) => e,
            }
        };
        if let Error::DatabaseError(DatabaseErrorKind::SerializationFailure, msg) = err {
            if let AnsiTransactionManager {
                status: TransactionManagerStatus::Valid(ref mut valid),
            } = self.transaction_state
            {
                valid.previous_error_relevant_for_rollback = Some((
                    DatabaseErrorKind::SerializationFailure,
                    msg.message().to_owned(),
                ))
            }
            Err(Error::DatabaseError(
                DatabaseErrorKind::SerializationFailure,
                msg,
            ))
        } else {
            Err(err)
        }
    }

    #[doc(hidden)]
    fn transaction_state(&mut self) -> &mut AnsiTransactionManager {
        &mut self.transaction_state
    }
}

#[cfg(feature = "r2d2")]
impl crate::r2d2::R2D2Connection for MysqlConnection {
    fn ping(&mut self) -> QueryResult<()> {
        crate::r2d2::CheckConnectionQuery.execute(self).map(|_| ())
    }

    fn is_broken(&mut self) -> bool {
        self.transaction_state
            .status
            .transaction_depth()
            .map(|d| d.is_some())
            .unwrap_or(true)
    }
}

impl MysqlConnection {
    fn prepared_query<'a, T: QueryFragment<Mysql> + QueryId>(
        &'a mut self,
        source: &'_ T,
    ) -> QueryResult<MaybeCached<'a, Statement>> {
        let cache = &mut self.statement_cache;
        let conn = &mut self.raw_connection;

        let mut stmt = cache.cached_statement(source, &Mysql, &[], |sql, _| conn.prepare(sql))?;
        let mut bind_collector = RawBytesBindCollector::new();
        source.collect_binds(&mut bind_collector, &mut (), &Mysql)?;
        let binds = bind_collector
            .metadata
            .into_iter()
            .zip(bind_collector.binds);
        stmt.bind(binds)?;
        Ok(stmt)
    }

    fn set_config_options(&mut self) -> QueryResult<()> {
        crate::sql_query("SET sql_mode=(SELECT CONCAT(@@sql_mode, ',PIPES_AS_CONCAT'))")
            .execute(self)?;
        crate::sql_query("SET time_zone = '+00:00';").execute(self)?;
        crate::sql_query("SET character_set_client = 'utf8mb4'").execute(self)?;
        crate::sql_query("SET character_set_connection = 'utf8mb4'").execute(self)?;
        crate::sql_query("SET character_set_results = 'utf8mb4'").execute(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate dotenvy;

    use super::*;
    use std::env;

    fn connection() -> MysqlConnection {
        dotenvy::dotenv().ok();
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
        assert!(crate::sql_query("SELECT 1").execute(connection).is_ok());
        assert!(crate::sql_query("SELECT 1").execute(connection).is_ok());
    }

    #[test]
    fn check_client_found_rows_flag() {
        let conn = &mut crate::test_helpers::connection();
        crate::sql_query("DROP TABLE IF EXISTS update_test CASCADE")
            .execute(conn)
            .unwrap();

        crate::sql_query("CREATE TABLE update_test(id INTEGER PRIMARY KEY, num INTEGER NOT NULL)")
            .execute(conn)
            .unwrap();

        crate::sql_query("INSERT INTO update_test(id, num) VALUES (1, 5)")
            .execute(conn)
            .unwrap();

        let output = crate::sql_query("UPDATE update_test SET num = 5 WHERE id = 1")
            .execute(conn)
            .unwrap();

        assert_eq!(output, 1);
    }
}
