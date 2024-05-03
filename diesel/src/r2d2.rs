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
//! use diesel::prelude::*;
//! use diesel::r2d2::ConnectionManager;
//! # use diesel::r2d2::CustomizeConnection;
//! # use diesel::r2d2::Error as R2D2Error;
//! use diesel::r2d2::Pool;
//! use diesel::result::Error;
//! use std::thread;
//!
//! # #[derive(Copy, Clone, Debug)]
//! # pub struct SetupUserTableCustomizer;
//! #
//! # impl CustomizeConnection<DbConnection, R2D2Error> for SetupUserTableCustomizer
//! # {
//! #     fn on_acquire(&self, conn: &mut DbConnection) -> Result<(), R2D2Error> {
//! #         setup_database(conn);
//! #         Ok(())
//! #     }
//! # }
//!
//! pub fn get_connection_pool() -> Pool<ConnectionManager<DbConnection>> {
//!     let url = database_url_for_env();
//!     let manager = ConnectionManager::<DbConnection>::new(url);
//!     // Refer to the `r2d2` documentation for more methods to use
//!     // when building a connection pool
//!     Pool::builder()
//! #         .max_size(1)
//!         .test_on_check_out(true)
//! #         .connection_customizer(Box::new(SetupUserTableCustomizer))
//!         .build(manager)
//!         .expect("Could not build connection pool")
//! }
//!
//! pub fn create_user(conn: &mut DbConnection, user_name: &str) -> Result<usize, Error> {
//!     use schema::users::dsl::*;
//!
//!     diesel::insert_into(users)
//!         .values(name.eq(user_name))
//!         .execute(conn)
//! }
//!
//! fn main() {
//!     let pool = get_connection_pool();
//!     let mut threads = vec![];
//!     let max_users_to_create = 1;
//!
//!     for i in 0..max_users_to_create {
//!         let pool = pool.clone();
//!         threads.push(thread::spawn({
//!             move || {
//!                 let conn = &mut pool.get().unwrap();
//!                 let name = format!("Person {}", i);
//!                 create_user(conn, &name).unwrap();
//!             }
//!         }))
//!     }
//!
//!     for handle in threads {
//!         handle.join().unwrap();
//!     }
//! }
//! ```
//!
//! # A note on error handling
//!
//! When used inside a pool, if an individual connection becomes
//! broken (as determined by the [R2D2Connection::is_broken] method)
//! then, when the connection goes out of scope, `r2d2` will close
//! and return the connection to the DB.
//!
//! `diesel` determines broken connections by whether or not the current
//! thread is panicking or if individual `Connection` structs are
//! broken (determined by the `is_broken()` method). Generically, these
//! are left to individual backends to implement themselves.
//!
//! For SQLite, PG, and MySQL backends `is_broken()` is determined
//! by whether or not the `TransactionManagerStatus` (as a part
//! of the `AnsiTransactionManager` struct) is in an `InError` state
//! or contains an open transaction when the connection goes out of scope.
//!

pub use r2d2::*;

/// A re-export of [`r2d2::Error`], which is only used by methods on [`r2d2::Pool`].
///
/// [`r2d2::Error`]: r2d2::Error
/// [`r2d2::Pool`]: r2d2::Pool
pub type PoolError = r2d2::Error;

use std::fmt;
use std::marker::PhantomData;

use crate::backend::Backend;
use crate::connection::{
    ConnectionSealed, LoadConnection, SimpleConnection, TransactionManager,
    TransactionManagerStatus,
};
use crate::expression::QueryMetadata;
use crate::prelude::*;
use crate::query_builder::{Query, QueryFragment, QueryId};

/// An r2d2 connection manager for use with Diesel.
///
/// See the [r2d2 documentation](https://docs.rs/r2d2/latest/r2d2/) for usage examples.
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

#[allow(unsafe_code)] // we do not actually hold a reference to `T`
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

    /// Modifies the URL which was supplied at initialization.
    ///
    /// This does not update any state for existing connections,
    /// but this new URL is used for new connections that are created.
    pub fn update_database_url<S: Into<String>>(&mut self, database_url: S) {
        self.database_url = database_url.into();
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
        (**self).batch_execute(query)
    }
}

impl<M> ConnectionSealed for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: ConnectionSealed,
{
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

    fn begin_test_transaction(&mut self) -> QueryResult<()> {
        (**self).begin_test_transaction()
    }

    fn execute_returning_count<T>(&mut self, source: &T) -> QueryResult<usize>
    where
        T: QueryFragment<Self::Backend> + QueryId,
    {
        (**self).execute_returning_count(source)
    }

    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData {
        (**self).transaction_state()
    }

    fn instrumentation(&mut self) -> &mut dyn crate::connection::Instrumentation {
        (**self).instrumentation()
    }

    fn set_instrumentation(&mut self, instrumentation: impl crate::connection::Instrumentation) {
        (**self).set_instrumentation(instrumentation)
    }
}

impl<B, M> LoadConnection<B> for PooledConnection<M>
where
    M: ManageConnection,
    M::Connection: LoadConnection<B> + R2D2Connection,
{
    type Cursor<'conn, 'query> = <M::Connection as LoadConnection<B>>::Cursor<'conn, 'query>;
    type Row<'conn, 'query> = <M::Connection as LoadConnection<B>>::Row<'conn, 'query>;

    fn load<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> QueryResult<Self::Cursor<'conn, 'query>>
    where
        T: Query + QueryFragment<Self::Backend> + QueryId + 'query,
        Self::Backend: QueryMetadata<T::SqlType>,
    {
        (**self).load(source)
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
        (**self).setup()
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
        (**self).update_and_fetch(changeset)
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

    #[test]
    fn check_pool_does_actually_hold_connections() {
        use std::sync::atomic::{AtomicU32, Ordering};

        #[derive(Debug)]
        struct TestEventHandler {
            acquire_count: Arc<AtomicU32>,
            release_count: Arc<AtomicU32>,
            checkin_count: Arc<AtomicU32>,
            checkout_count: Arc<AtomicU32>,
        }

        impl r2d2::HandleEvent for TestEventHandler {
            fn handle_acquire(&self, _event: r2d2::event::AcquireEvent) {
                self.acquire_count.fetch_add(1, Ordering::Relaxed);
            }
            fn handle_release(&self, _event: r2d2::event::ReleaseEvent) {
                self.release_count.fetch_add(1, Ordering::Relaxed);
            }
            fn handle_checkout(&self, _event: r2d2::event::CheckoutEvent) {
                self.checkout_count.fetch_add(1, Ordering::Relaxed);
            }
            fn handle_checkin(&self, _event: r2d2::event::CheckinEvent) {
                self.checkin_count.fetch_add(1, Ordering::Relaxed);
            }
        }

        let acquire_count = Arc::new(AtomicU32::new(0));
        let release_count = Arc::new(AtomicU32::new(0));
        let checkin_count = Arc::new(AtomicU32::new(0));
        let checkout_count = Arc::new(AtomicU32::new(0));

        let handler = Box::new(TestEventHandler {
            acquire_count: acquire_count.clone(),
            release_count: release_count.clone(),
            checkin_count: checkin_count.clone(),
            checkout_count: checkout_count.clone(),
        });

        let manager = ConnectionManager::<TestConnection>::new(database_url());
        let pool = Pool::builder()
            .max_size(1)
            .test_on_check_out(true)
            .event_handler(handler)
            .build(manager)
            .unwrap();

        assert_eq!(acquire_count.load(Ordering::Relaxed), 1);
        assert_eq!(release_count.load(Ordering::Relaxed), 0);
        assert_eq!(checkin_count.load(Ordering::Relaxed), 0);
        assert_eq!(checkout_count.load(Ordering::Relaxed), 0);

        // check that we reuse connections with the pool
        {
            let conn = pool.get().unwrap();

            assert_eq!(acquire_count.load(Ordering::Relaxed), 1);
            assert_eq!(release_count.load(Ordering::Relaxed), 0);
            assert_eq!(checkin_count.load(Ordering::Relaxed), 0);
            assert_eq!(checkout_count.load(Ordering::Relaxed), 1);
            std::mem::drop(conn);
        }

        assert_eq!(acquire_count.load(Ordering::Relaxed), 1);
        assert_eq!(release_count.load(Ordering::Relaxed), 0);
        assert_eq!(checkin_count.load(Ordering::Relaxed), 1);
        assert_eq!(checkout_count.load(Ordering::Relaxed), 1);

        // check that we remove a connection with open transactions from the pool
        {
            let mut conn = pool.get().unwrap();

            assert_eq!(acquire_count.load(Ordering::Relaxed), 1);
            assert_eq!(release_count.load(Ordering::Relaxed), 0);
            assert_eq!(checkin_count.load(Ordering::Relaxed), 1);
            assert_eq!(checkout_count.load(Ordering::Relaxed), 2);

            <TestConnection as Connection>::TransactionManager::begin_transaction(&mut *conn)
                .unwrap();
        }

        // we are not interested in the acquire count here
        // as the pool opens a new connection in the background
        // that could lead to this test failing if that happens to fast
        // (which is sometimes the case for sqlite)
        //assert_eq!(acquire_count.load(Ordering::Relaxed), 1);
        assert_eq!(release_count.load(Ordering::Relaxed), 1);
        assert_eq!(checkin_count.load(Ordering::Relaxed), 2);
        assert_eq!(checkout_count.load(Ordering::Relaxed), 2);

        // check that we remove a connection from the pool that was
        // open during panicking
        #[allow(unreachable_code, unused_variables)]
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let conn = pool.get();
            assert_eq!(acquire_count.load(Ordering::Relaxed), 2);
            assert_eq!(release_count.load(Ordering::Relaxed), 1);
            assert_eq!(checkin_count.load(Ordering::Relaxed), 2);
            assert_eq!(checkout_count.load(Ordering::Relaxed), 3);
            panic!();
            std::mem::drop(conn);
        }))
        .unwrap_err();

        // we are not interested in the acquire count here
        // as the pool opens a new connection in the background
        // that could lead to this test failing if that happens to fast
        // (which is sometimes the case for sqlite)
        //assert_eq!(acquire_count.load(Ordering::Relaxed), 2);
        assert_eq!(release_count.load(Ordering::Relaxed), 2);
        assert_eq!(checkin_count.load(Ordering::Relaxed), 3);
        assert_eq!(checkout_count.load(Ordering::Relaxed), 3);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn verify_that_begin_test_transaction_works_with_pools() {
        use crate::prelude::*;
        use crate::r2d2::*;

        table! {
            users {
                id -> Integer,
                name -> Text,
            }
        }

        #[derive(Debug)]
        struct TestConnectionCustomizer;

        impl<E> CustomizeConnection<PgConnection, E> for TestConnectionCustomizer {
            fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), E> {
                conn.begin_test_transaction()
                    .expect("Failed to start test transaction");

                Ok(())
            }
        }

        let manager = ConnectionManager::<PgConnection>::new(database_url());
        let pool = Pool::builder()
            .max_size(1)
            .connection_customizer(Box::new(TestConnectionCustomizer))
            .build(manager)
            .unwrap();

        let mut conn = pool.get().unwrap();

        crate::sql_query(
            "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
        )
        .execute(&mut conn)
        .unwrap();

        crate::insert_into(users::table)
            .values(users::name.eq("John"))
            .execute(&mut conn)
            .unwrap();

        std::mem::drop(conn);

        let mut conn2 = pool.get().unwrap();

        let user_count = users::table.count().get_result::<i64>(&mut conn2).unwrap();
        assert_eq!(user_count, 1);
    }
}
