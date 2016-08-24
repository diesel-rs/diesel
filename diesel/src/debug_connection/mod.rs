use connection::{SimpleConnection, Connection};
use result::*;
use query_builder::{QueryFragment, QueryId};
use types::HasSqlType;
use query_source::Queryable;
#[cfg(feature = "log")]
use connection::DebugSql;

#[cfg(not(feature = "log"))]
#[macro_use]
mod no_log{
    #[macro_export]
    macro_rules! debug {
        (target: $target:expr, $($arg:tt)*) => {};
        ($($arg:tt)*) => {};
    }

}


/// Wrapper to add logging statements to a connection. When calling a
/// function on this Connection there will be a call to the debug! macro
/// of the log crate. Logging is only enabled if diesel is build with the log
/// feature flag.
///
/// Example usage
/// -------------
///
/// ```rust
/// # extern crate diesel;
/// # #[cfg(feature = "postgres")]
/// # type ActualConnection = ::diesel::pg::PgConnection;
/// # #[cfg(not(feature = "postgres"))]
/// # type ActualConnection = ::diesel::sqlite::SqliteConnection;
/// # use diesel::Connection;
/// # fn main() {
/// #     use diesel::debug_connection::DebugConnection;
/// let conn = DebugConnection::<ActualConnection>::establish("your-database-url");
/// // Use the debug connection normally
/// # }
/// ```
#[allow(missing_debug_implementations)]
pub struct DebugConnection<Conn: Connection> {
    inner_connection: Conn,
}


impl<Conn: Connection> SimpleConnection for DebugConnection<Conn> {
    fn batch_execute(&self, query: &str) -> QueryResult<()> {
        debug!("batch execute: {}", query);
        self.inner_connection.batch_execute(query)
    }
}

impl<Conn: Connection> Connection for DebugConnection<Conn> {
    type Backend = Conn::Backend;
    type RawConnection = Conn::RawConnection;
    type PreparedQuery = Conn::PreparedQuery;

    fn establish(database_url: &str) -> ConnectionResult<Self> {
        debug!("establish connection to {}", database_url);
        let inner = try!(Conn::establish(database_url));
        Ok(DebugConnection { inner_connection: inner })
    }

    fn execute(&self, query: &str) -> QueryResult<usize> {
        debug!("execute query: {}", query);
        self.inner_connection.execute(query)
    }

    fn _query_all<ST, U>(&self, prepared: Self::PreparedQuery) -> QueryResult<Vec<U>>  where
        Self::Backend: HasSqlType<ST>,
        U: Queryable<ST, Self::Backend>
    {
        debug!("QueryAll: {}", prepared.get_debug_sql(self._get_raw_connection()));
        self.inner_connection._query_all::<ST, U>(prepared)
    }

    fn _execute_returning_count(&self, prepared: Self::PreparedQuery) -> QueryResult<usize>
    {
        debug!("execute_returing_count: {}", prepared.get_debug_sql(self._get_raw_connection()));
        self.inner_connection._execute_returning_count(prepared)
    }

    fn silence_notices<F: FnOnce() -> T, T>(&self, f: F) -> T {
        self.inner_connection.silence_notices(f)
    }

    fn begin_transaction(&self) -> QueryResult<()> {
        debug!("begin transaction!");
        self.inner_connection.begin_transaction()
    }

    fn rollback_transaction(&self) -> QueryResult<()> {
        debug!("rollback transaction!");
        self.inner_connection.rollback_transaction()
    }

    fn commit_transaction(&self) -> QueryResult<()> {
        debug!("commit transaction!");
        self.inner_connection.commit_transaction()
    }

    fn get_transaction_depth(&self) -> i32 {
        let res = self.inner_connection.get_transaction_depth();
        debug!("get transaction depth: {}", res);
        res
    }

    fn setup_helper_functions(&self) {
        debug!("setup helper functions! {}", "todo what?");
        self.inner_connection.setup_helper_functions();
    }

    fn prepare_query<T: QueryFragment<Self::Backend> + QueryId>(&self, source: &T)
        -> QueryResult<Self::PreparedQuery> {
        self.inner_connection.prepare_query(source)
    }

    fn _get_raw_connection(&self) -> &Self::RawConnection {
        self.inner_connection._get_raw_connection()
    }
}
