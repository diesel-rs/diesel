use std::borrow::Cow;

use diesel::connection::statement_cache::QueryFragmentForCachedStatement;
use diesel::mysql::Mysql;
use diesel::prelude::QueryResult;

use crate::mysql_like::UrlHelper;
use crate::stmt_cache::QueryFragmentHelper;

use super::mysql_like::AsyncMysqlLikeConnection;
use super::mysql_like::MysqlLikeCancelToken;

/// The capability to request cancellation of in-progress queries on a
/// connection.
pub type MysqlCancelToken = MysqlLikeCancelToken<Mysql>;

/// A connection to a MySQL database. Connection URLs should be in the form
/// `mysql://[user[:password]@]host/database_name`
///
/// # Cancellation and timeouts
///
/// Dropping a query future before it completes, for example when a timeout
/// expires, can leave this connection with an unfinished operation. Such a
/// connection must not be reused: drop it and establish a new connection
/// instead. Cancelling the future does not guarantee that the server stopped
/// executing the query or rolled back its effects.
///
/// The connection pool integrations detect this state and discard the broken
/// connection when it is returned to the pool. Users managing connections
/// without a pool must handle this themselves whenever they cancel or time out
/// an operation.
pub type AsyncMysqlConnection = AsyncMysqlLikeConnection<Mysql>;

impl QueryFragmentForCachedStatement<Mysql> for QueryFragmentHelper {
    fn construct_sql(&self, _backend: &Mysql) -> QueryResult<String> {
        Ok(self.sql.clone())
    }

    fn is_safe_to_cache_prepared(&self, _backend: &Mysql) -> QueryResult<bool> {
        Ok(self.safe_to_cache)
    }

    fn parameter_count(&self, _backend: &Mysql) -> QueryResult<usize> {
        Ok(self.bind_count)
    }
}

impl UrlHelper for Mysql {
    //`mysql_async` checks this for us
    fn check_and_replace_schema<'a>(url: &'a str) -> Result<Cow<'a, str>, mysql_async::UrlError> {
        Ok(url.into())
    }
}
