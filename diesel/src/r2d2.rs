//! Connection pooling via r2d2.
//!
//! Note: This module requires enabling the `r2d2` feature
//!
//! # Example
//!
//! The below snippet is a contrived example emulating a web application,
//! where one would first initialize the pool in the `main()` function
//! (at the start of a long-running process). One would then pass this
//! pool struct around as shared state, which, here, we've emulated using
//! threads instead of routes.
//!
//! ```rust
//! # include!("doctest_setup.rs");
//! use diesel::result::Error;
//! use diesel::pg::PgConnection;
//! use diesel::prelude::*;
//! use diesel::r2d2::ConnectionManager;
//! use diesel::r2d2::Pool;
//! use std::env;
//! use std::sync::Arc;
//! use std::sync::Mutex;
//! use std::thread;
//!
//! pub type PostgresPool = Pool<ConnectionManager<PgConnection>>;
//!
//! pub fn get_connection_pool(pool_size: u32) -> PostgresPool {
//!     let url = database_url_for_env();
//!     let manager = ConnectionManager::<PgConnection>::new(url);
//!     // Refer to the `r2d2` documentation for more methods to use
//!     // when building a connection pool
//!     Pool::builder()
//!         .max_size(pool_size)
//!         .test_on_check_out(true)
//!         .build(manager)
//!         .expect("Could not build connection pool")
//! }
//!
//! pub fn create_user(conn: &mut PgConnection, user_name: &str) -> Result<usize, Error> {
//!     use schema::users::dsl::*;
//!
//!     diesel::insert_into(users)
//!         .values(name.eq(user_name))
//!         .execute(conn)
//! }
//!
//! pub fn setup_user_table(conn: &mut PgConnection) {
//!     diesel::sql_query("DROP TABLE IF EXISTS users CASCADE").execute(conn).unwrap();
//!     diesel::sql_query("CREATE TABLE users (
//!         id SERIAL PRIMARY KEY,
//!         name VARCHAR NOT NULL
//!     )")
//!         .execute(conn)
//!         .unwrap();
//! }
//!
//! pub fn delete_user_table(conn: &mut PgConnection) {
//!     diesel::sql_query("DROP TABLE IF EXISTS users CASCADE").execute(conn).unwrap();
//! }
//!
//! fn main() {
//!     let pool_size = 1;
//!     let connection_pool = Arc::new(Mutex::new(get_connection_pool(pool_size)));
//!     setup_user_table(&mut connection_pool.lock().unwrap().get().unwrap());
//!
//!     let mut threads = vec![];
//!     let max_users_to_create = 1;
//!
//!     for i in 0..max_users_to_create {
//!         let connection_pool = Arc::clone(&connection_pool);
//!         threads.push(thread::spawn({
//!             move || {
//!                 let connection_pool = connection_pool.lock().unwrap();
//!                 let conn = &mut connection_pool.get().unwrap();
//!                 let name = format!("Person {}", i);
//!                 create_user(conn, &name).unwrap();
//!             }
//!         }))
//!     }
//!
//!     for handle in threads {
//!         handle.join().unwrap();
//!     }
//!
//!     delete_user_table(&mut connection_pool.lock().unwrap().get().unwrap());
//! }
//! ```
//!
//! # A note on error handling
//!
//! When used inside a pool, if an individual connection becomes
//! broken (as determined by the [R2D2Connection::is_broken] method)
//! then `r2d2` will put close and return the connection to the DB.
//!
//! `diesel` determines broken connections by whether or not the current
//! thread is panicking or if individual `Connection` structs are
//! broken (determined by the `is_broken()` method). Generically, these
//! are left to individual backends to implement themselves.
//!
//! For SQLite, PG, and MySQL backends, specifically, `is_broken()`
//! is determined by whether or not the `TransactionManagerStatus` (as a part
//! of the `AnsiTransactionManager` struct) is in an `InError` state.
//!

pub use r2d2::*;

/// A re-export of [`r2d2::Error`], which is only used by methods on [`r2d2::Pool`].
///
/// [`r2d2::Error`]: r2d2::Error
/// [`r2d2::Pool`]: r2d2::Pool
pub type PoolError = r2d2::Error;

use std::convert::Into;
use std::fmt;
use std::marker::PhantomData;

use crate::backend::Backend;
use crate::connection::commit_error_processor::{CommitErrorOutcome, CommitErrorProcessor};
use crate::connection::{
    ConnectionGatWorkaround, SimpleConnection, TransactionManager, TransactionManagerStatus,
};
use crate::expression::QueryMetadata;
use crate::prelude::*;
use crate::query_builder::{Query, QueryFragment, QueryId};

/// An r2d2 connection manager for use with Diesel.
///
/// See the [r2d2 documentation] for usage examples.
///
/// [r2d2 documentation]: r2d2
#[derive(Clone)]
pub struct ConnectionManager<T> {
    database_url: String,
    _marker: PhantomData<T>,
}

impl<T> fmt::Debug for ConnectionManager<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConnectionManager<{}>", std::any::type_name::<T>())
    }
}

unsafe impl<T: Send + 'static> Sync for ConnectionManager<T> {}

impl<T> ConnectionManager<T> {
    /// Returns a new connection manager,
    /// which establishes connections to the given database URL.
    pub fn new<S: Into<String>>(database_url: S) -> Self {
        ConnectionManager {
            database_url: database_url.into(),
            _marker: PhantomData,
        }
    }
}

/// The error used when managing connections with `r2d2`.
#[derive(Debug)]
pub enum Error {
    /// An error occurred establishing the connection
    ConnectionError(ConnectionError),

    /// An error occurred pinging the database
    QueryError(crate::result::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::ConnectionError(ref e) => e.fmt(f),
            Error::QueryError(ref e) => e.fmt(f),
        }
    }
}

impl ::std::error::Error for Error {}

/// A trait indicating a connection could be used inside a r2d2 pool
pub trait R2D2Connection: Connection {
    /// Check if a connection is still valid
    fn ping(&mut self) -> QueryResult<()>;

    /// Checks if the connection is broken and should not be reused
    ///
    /// This method should return only contain a fast non-blocking check
    /// if the connection is considered to be broken or not. See
    /// [ManageConnection::has_broken] for details.
    ///
    /// The default implementation does not consider any connection as broken
    fn is_broken(&mut self) -> bool {
        false
    }
}

impl<T> ManageConnection for ConnectionManager<T>
where
    T: R2D2Connection + Send + 'static,
{
    type Connection = T;
    type Error = Error;

    fn connect(&self) -> Result<T, Error> {
        T::establish(&self.database_url).map_err(Error::ConnectionError)
    }

    fn is_valid(&self, conn: &mut T) -> Result<(), Error> {
        conn.ping().map_err(Error::QueryError)
    }

    fn has_broken(&self, conn: &mut T) -> bool {
        std::thread::panicking() || conn.is_broken()
    }
}

impl<M> SimpleConnection for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: R2D2Connection + Send + 'static,
{
    fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
        (&mut **self).batch_execute(query)
    }
}

impl<'conn, 'query, DB, M> ConnectionGatWorkaround<'conn, 'query, DB> for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: Connection<Backend = DB>,
    DB: Backend,
{
    type Cursor = <M::Connection as ConnectionGatWorkaround<'conn, 'query, DB>>::Cursor;
    type Row = <M::Connection as ConnectionGatWorkaround<'conn, 'query, DB>>::Row;
}

impl<M> CommitErrorProcessor for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: R2D2Connection + CommitErrorProcessor + Send + 'static,
{
    fn process_commit_error(&self, error: crate::result::Error) -> CommitErrorOutcome {
        (&**self).process_commit_error(error)
    }
}

impl<M> Connection for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: Connection + R2D2Connection + Send + 'static,
{
    type Backend = <M::Connection as Connection>::Backend;
    type TransactionManager =
        PoolTransactionManager<<M::Connection as Connection>::TransactionManager>;

    fn establish(_: &str) -> ConnectionResult<Self> {
        Err(ConnectionError::BadConnection(String::from(
            "Cannot directly establish a pooled connection",
        )))
    }

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<<Self as ConnectionGatWorkaround<'conn, 'query, Self::Backend>>::Cursor>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        (&mut **self).load(source)
    }

    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        (&mut **self).execute_returning_count(source)
    }

    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData {
        (&mut **self).transaction_state()
    }

    fn begin_test_transaction(&mut self) -> QueryResult<()> {
        (&mut **self).begin_test_transaction()
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct PoolTransactionManager<T>(std::marker::PhantomData<T>);

impl<M, T> TransactionManager<PooledConnection<M>> for PoolTransactionManager<T>
where
    M: ManageConnection,
    M::Connection: Connection<TransactionManager = T> + R2D2Connection,
    T: TransactionManager<M::Connection>,
{
    type TransactionStateData = T::TransactionStateData;

    fn begin_transaction(conn: &mut PooledConnection<M>) -> QueryResult<()> {
        T::begin_transaction(&mut **conn)
    }

    fn rollback_transaction(conn: &mut PooledConnection<M>) -> QueryResult<()> {
        T::rollback_transaction(&mut **conn)
    }

    fn commit_transaction(conn: &mut PooledConnection<M>) -> QueryResult<()> {
        T::commit_transaction(&mut **conn)
    }

    fn transaction_manager_status_mut(
        conn: &mut PooledConnection<M>,
    ) -> &mut TransactionManagerStatus {
        T::transaction_manager_status_mut(&mut **conn)
    }
}

impl<M> crate::migration::MigrationConnection for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: crate::migration::MigrationConnection,
    Self: Connection,
{
    fn setup(&mut self) -> QueryResult<usize> {
        (&mut **self).setup()
    }
}

impl<Changes, Output, M> crate::query_dsl::UpdateAndFetchResults<Changes, Output>
    for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: crate::query_dsl::UpdateAndFetchResults<Changes, Output>,
    Self: Connection,
{
    fn update_and_fetch(&mut self, changeset: Changes) -> QueryResult<Output> {
        (&mut **self).update_and_fetch(changeset)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    use crate::r2d2::*;
    use crate::test_helpers::*;

    #[test]
    fn establish_basic_connection() {
        let manager = ConnectionManager::<TestConnection>::new(database_url());
        let pool = Arc::new(Pool::builder().max_size(2).build(manager).unwrap());

        let (s1, r1) = mpsc::channel();
        let (s2, r2) = mpsc::channel();

        let pool1 = Arc::clone(&pool);
        let t1 = thread::spawn(move || {
            let conn = pool1.get().unwrap();
            s1.send(()).unwrap();
            r2.recv().unwrap();
            drop(conn);
        });

        let pool2 = Arc::clone(&pool);
        let t2 = thread::spawn(move || {
            let conn = pool2.get().unwrap();
            s2.send(()).unwrap();
            r1.recv().unwrap();
            drop(conn);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        pool.get().unwrap();
    }

    #[test]
    fn is_valid() {
        let manager = ConnectionManager::<TestConnection>::new(database_url());
        let pool = Pool::builder()
            .max_size(1)
            .test_on_check_out(true)
            .build(manager)
            .unwrap();

        pool.get().unwrap();
    }

    #[test]
    fn pooled_connection_impls_connection() {
        use crate::select;
        use crate::sql_types::Text;

        let manager = ConnectionManager::<TestConnection>::new(database_url());
        let pool = Pool::builder()
            .max_size(1)
            .test_on_check_out(true)
            .build(manager)
            .unwrap();
        let mut conn = pool.get().unwrap();

        let query = select("foo".into_sql::<Text>());
        assert_eq!("foo", query.get_result::<String>(&mut conn).unwrap());
    }
}

#[derive(QueryId)]
pub(crate) struct CheckConnectionQuery;

impl<DB> QueryFragment<DB> for CheckConnectionQuery
where
    DB: Backend,
{
    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, DB>,
    ) -> QueryResult<()> {
        pass.push_sql("SELECT 1");
        Ok(())
    }
}

impl Query for CheckConnectionQuery {
    type SqlType = crate::sql_types::Integer;
}

impl<C> RunQueryDsl<C> for CheckConnectionQuery {}
