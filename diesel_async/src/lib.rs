#![cfg_attr(docsrs, feature(doc_cfg))]
//! Diesel-async provides async variants of diesel related query functionality
//!
//! diesel-async is an extension to diesel itself. It is designed to be used together
//! with the main diesel crate. It only provides async variants of core diesel traits,
//! that perform actual io-work.
//! This includes async counterparts the following traits:
//! * [`diesel::prelude::RunQueryDsl`](https://docs.diesel.rs/2.3.x/diesel/prelude/trait.RunQueryDsl.html)
//!   -> [`diesel_async::RunQueryDsl`](crate::RunQueryDsl)
//! * [`diesel::connection::Connection`](https://docs.diesel.rs/2.3.x/diesel/connection/trait.Connection.html)
//!   -> [`diesel_async::AsyncConnection`](crate::AsyncConnection)
//! * [`diesel::query_dsl::UpdateAndFetchResults`](https://docs.diesel.rs/2.3.x/diesel/query_dsl/trait.UpdateAndFetchResults.html)
//!   -> [`diesel_async::UpdateAndFetchResults`](crate::UpdateAndFetchResults)
//!
//! These traits closely mirror their diesel counter parts while providing async functionality.
//!
//! In addition to these core traits 3 fully async connection implementations are provided
//! by diesel-async:
//!
//! * [`AsyncMysqlConnection`] (enabled by the `mysql` feature)
//! * [`AsyncPgConnection`] (enabled by the `postgres` feature)
//! * [`SyncConnectionWrapper`](sync_connection_wrapper::SyncConnectionWrapper) (enabled by the `sync-connection-wrapper`/`sqlite` feature)
//!
//! Ordinary usage of `diesel-async` assumes that you just replace the corresponding sync trait
//! method calls and connections with their async counterparts.
//!
//! ```rust
//! # include!("./doctest_setup.rs");
//! #
//! use diesel::prelude::*;
//! use diesel_async::{RunQueryDsl, AsyncConnection};
//!
//! diesel::table! {
//!    users(id) {
//!        id -> Integer,
//!        name -> Text,
//!    }
//! }
//! #
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! #     run_test().await;
//! # }
//! #
//! # async fn run_test() -> QueryResult<()> {
//!
//! use crate::users::dsl::*;
//!
//! # let mut connection = establish_connection().await;
//! # /*
//! let mut connection = AsyncPgConnection::establish(std::env::var("DATABASE_URL")?).await?;
//! # */
//! let data = users
//!     // use ordinary diesel query dsl here
//!     .filter(id.gt(0))
//!     // execute the query via the provided
//!     // async variant of `diesel_async::RunQueryDsl`
//!     .load::<(i32, String)>(&mut connection)
//!     .await?;
//! let expected_data = vec![
//!     (1, String::from("Sean")),
//!     (2, String::from("Tess")),
//! ];
//! assert_eq!(expected_data, data);
//! #     Ok(())
//! # }
//! ```
//!
//! ## Crate features:
//!
//! * `postgres`: Enables the [`AsyncPgConnection`] implementation
//! * `mysql`: Enables the [`AsyncMysqlConnection`] implementation
//! * `sqlite`: Enables the [`SyncConnectionWrapper`](crate::sync_connection_wrapper::SyncConnectionWrapper)
//!   and everything required to work with SQLite
//! * `sync-connection-wrapper`: Enables the
//!   [`SyncConnectionWrapper`](crate::sync_connection_wrapper::SyncConnectionWrapper) which allows to
//!   wrap sync connections from [`diesel`] into async connection wrapper
//! * `async-connection-wrapper`: Enables the [`AsyncConnectionWrapper`](crate::async_connection_wrapper::AsyncConnectionWrapper)
//!   which allows
//!   to use connection implementations from this crate as sync [`diesel::Connection`]
//! * `migrations`: Enables the [`AsyncMigrationHarness`] to execute migrations via
//!   [`diesel_migrations`]
//! * `pool`: Enables general support for connection pools
//! * `r2d2`: Enables support for pooling via the [`r2d2`] crate
//! * `bb8`: Enables support for pooling via the [`bb8`] crate
//! * `mobc`: Enables support for pooling via the [`mobc`] crate
//! * `deadpool`: Enables support for pooling via the [`deadpool`] crate

#![warn(
    missing_docs,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use diesel::backend::Backend;
use diesel::connection::{CacheSize, Instrumentation};
use diesel::query_builder::{AsQuery, QueryFragment, QueryId};
use diesel::row::Row;
use diesel::sql_types::TypeMetadata;
use diesel::{ConnectionResult, QueryResult};
use futures_core::Stream;
use futures_util::FutureExt;
use std::fmt::Debug;
use std::future::Future;
use transaction_manager::AsyncFunc;

#[cfg(feature = "async-connection-wrapper")]
pub mod async_connection_wrapper;
mod deref_connection;
#[cfg(feature = "mariadb")]
mod mariadb;
#[cfg(feature = "migrations")]
mod migrations;
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(any(feature = "mysql", feature = "mariadb"))]
mod mysql_like;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "pool")]
pub mod pooled_connection;
mod run_query_dsl;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "mariadb"))]
mod stmt_cache;
#[cfg(feature = "sync-connection-wrapper")]
pub mod sync_connection_wrapper;
mod transaction_manager;

#[cfg(feature = "mariadb")]
#[doc(inline)]
pub use self::mariadb::AsyncMariadbConnection;
#[cfg(feature = "mariadb")]
#[doc(inline)]
pub use self::mariadb::MariadbCancelToken;
#[cfg(feature = "mysql")]
#[doc(inline)]
pub use self::mysql::AsyncMysqlConnection;
#[cfg(feature = "mysql")]
#[doc(inline)]
pub use self::mysql::MysqlCancelToken;
#[cfg(feature = "postgres")]
#[doc(inline)]
pub use self::pg::AsyncPgConnection;
#[doc(inline)]
pub use self::run_query_dsl::*;

#[doc(inline)]
#[cfg(feature = "migrations")]
pub use self::migrations::AsyncMigrationHarness;
#[doc(inline)]
pub use self::transaction_manager::{AnsiTransactionManager, TransactionManager};

/// Perform simple operations on a backend.
///
/// You should likely use [`AsyncConnection`] instead.
pub trait SimpleAsyncConnection {
    /// Execute multiple SQL statements within the same string.
    ///
    /// This function is used to execute migrations,
    /// which may contain more than one SQL statement.
    fn batch_execute(&mut self, query: &str) -> impl Future<Output = QueryResult<()>> + Send;
}

/// Core trait for an async database connection
pub trait AsyncConnectionCore: SimpleAsyncConnection + Send {
    /// The future returned by `AsyncConnection::execute`
    type ExecuteFuture<'conn, 'query>: Future<Output = QueryResult<usize>> + Send;
    /// The future returned by `AsyncConnection::load`
    type LoadFuture<'conn, 'query>: Future<Output = QueryResult<Self::Stream<'conn, 'query>>> + Send;
    /// The inner stream returned by `AsyncConnection::load`
    type Stream<'conn, 'query>: Stream<Item = QueryResult<Self::Row<'conn, 'query>>> + Send;
    /// The row type used by the stream returned by `AsyncConnection::load`
    type Row<'conn, 'query>: Row<'conn, Self::Backend>;

    /// The backend this type connects to
    type Backend: Backend;

    #[doc(hidden)]
    fn load<'conn, 'query, T>(&'conn mut self, source: T) -> Self::LoadFuture<'conn, 'query>
    where
        T: AsQuery + 'query,
        T::Query: QueryFragment<Self::Backend> + QueryId + 'query;

    #[doc(hidden)]
    fn execute_returning_count<'conn, 'query, T>(
        &'conn mut self,
        source: T,
    ) -> Self::ExecuteFuture<'conn, 'query>
    where
        T: QueryFragment<Self::Backend> + QueryId + 'query;

    // These functions allow the associated types (`ExecuteFuture`, `LoadFuture`, etc.) to
    // compile without a `where Self: '_` clause. This is needed the because bound causes
    // lifetime issues when using `transaction()` with generic `AsyncConnection`s.
    //
    // See: https://github.com/rust-lang/rust/issues/87479
    #[doc(hidden)]
    fn _silence_lint_on_execute_future(_: Self::ExecuteFuture<'_, '_>) {}
    #[doc(hidden)]
    fn _silence_lint_on_load_future(_: Self::LoadFuture<'_, '_>) {}
}

/// An async connection to a database
///
/// This trait represents an async database connection. It can be used to query the database through
/// the query dsl provided by diesel, custom extensions or raw sql queries. It essentially mirrors
/// the sync diesel [`Connection`](diesel::connection::Connection) implementation
pub trait AsyncConnection: AsyncConnectionCore + Sized {
    #[doc(hidden)]
    type TransactionManager: TransactionManager<Self>;

    /// Establishes a new connection to the database
    ///
    /// The argument to this method and the method's behavior varies by backend.
    /// See the documentation for that backend's connection class
    /// for details about what it accepts and how it behaves.
    fn establish(database_url: &str) -> impl Future<Output = ConnectionResult<Self>> + Send;

    /// Executes the given function inside of a database transaction
    ///
    /// This function executes the provided closure `f` inside a database
    /// transaction. If there is already an open transaction for the current
    /// connection savepoints will be used instead. The connection is committed if
    /// the closure returns `Ok(_)`, it will be rolled back if it returns `Err(_)`.
    /// For both cases the original result value will be returned from this function.
    ///
    /// If the transaction fails to commit due to a `SerializationFailure` or a
    /// `ReadOnlyTransaction` a rollback will be attempted.
    /// If the rollback fails, the error will be returned in a
    /// [`Error::RollbackErrorOnCommit`](diesel::result::Error::RollbackErrorOnCommit),
    /// from which you will be able to extract both the original commit error and
    /// the rollback error.
    /// In addition, the connection will be considered broken
    /// as it contains a uncommitted unabortable open transaction. Any further
    /// interaction with the transaction system will result in an returned error
    /// in this case.
    ///
    /// If the closure returns an `Err(_)` and the rollback fails the function
    /// will return that rollback error directly, and the transaction manager will
    /// be marked as broken as it contains a uncommitted unabortable open transaction.
    ///
    /// If a nested transaction fails to release the corresponding savepoint
    /// the error will be returned directly.
    ///
    /// **WARNING:** Canceling the returned future does currently **not**
    /// close an already open transaction. You may end up with a connection
    /// containing a dangling transaction.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// use diesel::result::Error;
    /// use diesel_async::{RunQueryDsl, AsyncConnection};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// #     run_test().await.unwrap();
    /// # }
    /// #
    /// # async fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection().await;
    /// conn.transaction::<_, Error, _>(async |conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)
    ///         .await?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn).await?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// }).await?;
    ///
    /// conn.transaction::<(), _, _>(async |conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Pascal"))
    ///         .execute(conn)
    ///         .await?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn).await?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby", "Pascal"], all_names);
    ///
    ///     // If we want to roll back the transaction, but don't have an
    ///     // actual error to return, we can return `RollbackTransaction`.
    ///     Err(Error::RollbackTransaction)
    /// }).await;
    ///
    /// let all_names = users.select(name).load::<String>(conn).await?;
    /// assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn transaction<'a, 'conn, R, E, F>(
        &'conn mut self,
        callback: F,
    ) -> impl Future<Output = Result<R, E>> + Send + 'conn
    where
        for<'r> F: AsyncFnOnce(&'r mut Self) -> Result<R, E>
            + AsyncFunc<&'r mut Self, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: From<diesel::result::Error> + Send + 'a,
        R: Send + 'a,
        'a: 'conn,
    {
        Self::TransactionManager::transaction(self, callback)
    }

    /// Creates a transaction that will never be committed. This is useful for
    /// tests. Panics if called while inside of a transaction or
    /// if called with a connection containing a broken transaction
    fn begin_test_transaction(&mut self) -> impl Future<Output = QueryResult<()>> + Send {
        use diesel::connection::TransactionManagerStatus;

        async {
            match Self::TransactionManager::transaction_manager_status_mut(self) {
                TransactionManagerStatus::Valid(valid_status) => {
                    assert_eq!(None, valid_status.transaction_depth())
                }
                TransactionManagerStatus::InError => panic!("Transaction manager in error"),
            };
            Self::TransactionManager::begin_transaction(self).await?;
            // set the test transaction flag
            // to prevent that this connection gets dropped in connection pools
            // Tests commonly set the poolsize to 1 and use `begin_test_transaction`
            // to prevent modifications to the schema
            Self::TransactionManager::transaction_manager_status_mut(self)
                .set_test_transaction_flag();
            Ok(())
        }
    }

    /// Executes the given function inside a transaction, but does not commit
    /// it. Panics if the given function returns an error.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// use diesel::result::Error;
    /// use diesel_async::{RunQueryDsl, AsyncConnection};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// #     run_test().await.unwrap();
    /// # }
    /// #
    /// # async fn run_test() -> QueryResult<()> {
    /// #     use schema::users::dsl::*;
    /// #     let conn = &mut establish_connection().await;
    /// conn.test_transaction::<_, Error, _>(async |conn| {
    ///     diesel::insert_into(users)
    ///         .values(name.eq("Ruby"))
    ///         .execute(conn)
    ///         .await?;
    ///
    ///     let all_names = users.select(name).load::<String>(conn).await?;
    ///     assert_eq!(vec!["Sean", "Tess", "Ruby"], all_names);
    ///
    ///     Ok(())
    /// }).await;
    ///
    /// // Even though we returned `Ok`, the transaction wasn't committed.
    /// let all_names = users.select(name).load::<String>(conn).await?;
    /// assert_eq!(vec!["Sean", "Tess"], all_names);
    /// #     Ok(())
    /// # }
    /// ```
    fn test_transaction<'conn, 'a, R, E, F>(
        &'conn mut self,
        f: F,
    ) -> impl Future<Output = R> + Send + 'conn
    where
        for<'r> F: AsyncFnOnce(&'r mut Self) -> Result<R, E>
            + AsyncFunc<&'r mut Self, Result<R, E>, Fut: Send>
            + Send
            + 'a,
        E: Debug + Send + 'a,
        R: Send + 'a,
        'a: 'conn,
    {
        use futures_util::TryFutureExt;
        let (user_result_tx, user_result_rx) = std::sync::mpsc::channel();
        self.transaction::<R, _, _>(async |conn| {
            f(conn)
                .map_err(|_| diesel::result::Error::RollbackTransaction)
                .and_then(move |r| {
                    let _ = user_result_tx.send(r);
                    std::future::ready(Err(diesel::result::Error::RollbackTransaction))
                })
                .await
        })
        .then(move |_r| {
            let r = user_result_rx
                .try_recv()
                .expect("Transaction did not succeed");
            std::future::ready(r)
        })
    }

    #[doc(hidden)]
    fn transaction_state(
        &mut self,
    ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData;

    #[doc(hidden)]
    fn instrumentation(&mut self) -> &mut dyn Instrumentation;

    /// Set a specific [`Instrumentation`] implementation for this connection
    fn set_instrumentation(&mut self, instrumentation: impl Instrumentation);

    /// Set the prepared statement cache size to [`CacheSize`] for this connection
    fn set_prepared_statement_cache_size(&mut self, size: CacheSize);
}

/// `AsyncMultiConnectionHelper` is an async copy of `MultiConnectionHelper` trait.
/// It is used to support async MultiConnection implementations.
pub trait AsyncMultiConnectionHelper: AsyncConnectionCore {
    /// Convert the lookup type to any
    fn to_any<'a>(
        lookup: &mut <Self::Backend as TypeMetadata>::MetadataLookup,
    ) -> &mut (dyn std::any::Any + 'a);

    /// Get the lookup type from any
    fn from_any(
        lookup: &mut dyn std::any::Any,
    ) -> Option<&mut <Self::Backend as TypeMetadata>::MetadataLookup>;
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "pool")]
macro_rules! expand_pool {
        ($($tt:tt)*) => {$($tt)*};
    }

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "pool"))]
macro_rules! expand_pool {
    ($($tt:tt)*) => {};
}
