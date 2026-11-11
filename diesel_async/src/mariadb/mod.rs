use std::borrow::Cow;

use diesel::connection::statement_cache::QueryFragmentForCachedStatement;
use diesel::mariadb::Mariadb;
use diesel::prelude::QueryResult;

use crate::mysql_like::UrlHelper;
use crate::stmt_cache::QueryFragmentHelper;

use super::mysql_like::AsyncMysqlLikeConnection;
use super::mysql_like::MysqlLikeCancelToken;

/// The capability to request cancellation of in-progress queries on a
/// connection.
pub type MariadbCancelToken = MysqlLikeCancelToken<Mariadb>;

/// A connection to a Mariadb database. Connection URLs should be in the form
/// `mariadb://[user[:password]@]host/database_name`
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
pub type AsyncMariadbConnection = AsyncMysqlLikeConnection<Mariadb>;

impl QueryFragmentForCachedStatement<Mariadb> for QueryFragmentHelper {
    fn construct_sql(&self, _backend: &Mariadb) -> QueryResult<String> {
        Ok(self.sql.clone())
    }

    fn is_safe_to_cache_prepared(&self, _backend: &Mariadb) -> QueryResult<bool> {
        Ok(self.safe_to_cache)
    }

    fn parameter_count(&self, _backend: &Mariadb) -> QueryResult<usize> {
        Ok(self.bind_count)
    }
}

impl UrlHelper for Mariadb {
    //`mysql_async` exprects `mysql` as schema
    fn check_and_replace_schema<'a>(url: &'a str) -> Result<Cow<'a, str>, mysql_async::UrlError> {
        let mut url = url::Url::parse(url)?;
        let scheme = url.scheme();
        if scheme != "mariadb" {
            return Err(mysql_async::UrlError::UnsupportedScheme {
                scheme: scheme.to_string(),
            });
        }
        let _ = url.set_scheme("mysql");
        Ok(url.to_string().into())
    }
}
