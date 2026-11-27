//! This module contains an wrapper type
//! that provides a [`diesel::Connection`]
//! implementation for types that implement
//! [`crate::AsyncConnection`]. Using this type
//! might be useful for the following usecases:
//!
//! * Executing migrations on application startup
//! * Using a pure rust diesel connection implementation
//!   as replacement for the existing connection
//!   implementations provided by diesel

use futures_core::Stream;
use futures_util::StreamExt;
use std::future::Future;
use std::pin::Pin;

/// This is a helper trait that allows to customize the
/// async runtime used to execute futures as part of the
/// [`AsyncConnectionWrapper`] type. By default a
/// tokio runtime is used.
pub trait BlockOn {
    /// This function should allow to execute a
    /// given future to get the result
    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future;

    /// This function should be used to construct
    /// a new runtime instance
    fn get_runtime() -> Self;
}

/// A helper type that wraps an [`AsyncConnection`][crate::AsyncConnection] to
/// provide a sync [`diesel::Connection`] implementation.
///
/// Internally this wrapper type will use `block_on` to wait for
/// the execution of futures from the inner connection. This implies you
/// cannot use functions of this type in a scope with an already existing
/// tokio runtime. If you are in a situation where you want to use this
/// connection wrapper in the scope of an existing tokio runtime (for example
/// for running migrations via `diesel_migration`) you need to wrap
/// the relevant code block into a `tokio::task::spawn_blocking` task.
///
/// # Examples
///
/// ```rust,no_run
/// # include!("doctest_setup.rs");
/// use schema::users;
/// use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// use diesel::prelude::{RunQueryDsl, Connection};
/// # let database_url = database_url();
/// let mut conn = AsyncConnectionWrapper::<DbConnection>::establish(&database_url)?;
///
/// let all_users = users::table.load::<(i32, String)>(&mut conn)?;
/// # assert_eq!(all_users.len(), 0);
/// # Ok(())
/// # }
/// ```
///
/// If you are in the scope of an existing tokio runtime you need to use
/// `tokio::task::spawn_blocking` to encapsulate the blocking tasks
/// ```rust,no_run
/// # include!("doctest_setup.rs");
/// use schema::users;
/// use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
///
/// async fn some_async_fn() {
/// # let database_url = database_url();
///      // need to use `spawn_blocking` to execute
///      // a blocking task in the scope of an existing runtime
///      let res = tokio::task::spawn_blocking(move || {
///          use diesel::prelude::{RunQueryDsl, Connection};
///          let mut conn = AsyncConnectionWrapper::<DbConnection>::establish(&database_url)?;
///
///          let all_users = users::table.load::<(i32, String)>(&mut conn)?;
/// #         assert_eq!(all_users.len(), 0);
///          Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
///      }).await;
///
/// # res.unwrap().unwrap();
/// }
///
/// # #[tokio::main]
/// # async fn main() {
/// #    some_async_fn().await;
/// # }
/// ```
#[cfg(feature = "tokio")]
pub type AsyncConnectionWrapper<C, B = self::implementation::Tokio> =
    self::implementation::AsyncConnectionWrapper<C, B>;

/// A helper type that wraps an [`crate::AsyncConnectionWrapper`] to
/// provide a sync [`diesel::Connection`] implementation.
///
/// Internally this wrapper type will use `block_on` to wait for
/// the execution of futures from the inner connection.
#[cfg(not(feature = "tokio"))]
pub use self::implementation::AsyncConnectionWrapper;

pub(crate) mod implementation {
    use diesel::connection::{CacheSize, Instrumentation, SimpleConnection};
    use std::ops::{Deref, DerefMut};

    use super::*;

    pub struct AsyncConnectionWrapper<C, B> {
        inner: C,
        runtime: B,
    }

    impl<C, B> From<C> for AsyncConnectionWrapper<C, B>
    where
        C: crate::AsyncConnection,
        B: BlockOn + Send,
    {
        fn from(inner: C) -> Self {
            Self {
                inner,
                runtime: B::get_runtime(),
            }
        }
    }

    impl<C, B> AsyncConnectionWrapper<C, B>
    where
        C: crate::AsyncConnection,
    {
        /// Consumes the [`AsyncConnectionWrapper`] returning the wrapped inner
        /// [`AsyncConnection`].
        pub fn into_inner(self) -> C {
            self.inner
        }
    }

    impl<C, B> Deref for AsyncConnectionWrapper<C, B> {
        type Target = C;

        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }

    impl<C, B> DerefMut for AsyncConnectionWrapper<C, B> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.inner
        }
    }

    impl<C, B> diesel::connection::SimpleConnection for AsyncConnectionWrapper<C, B>
    where
        C: crate::SimpleAsyncConnection,
        B: BlockOn,
    {
        fn batch_execute(&mut self, query: &str) -> diesel::QueryResult<()> {
            let f = self.inner.batch_execute(query);
            self.runtime.block_on(f)
        }
    }

    impl<C, B> diesel::connection::ConnectionSealed for AsyncConnectionWrapper<C, B> {}

    impl<C, B> diesel::connection::Connection for AsyncConnectionWrapper<C, B>
    where
        C: crate::AsyncConnection,
        B: BlockOn + Send,
    {
        type Backend = C::Backend;

        type TransactionManager = AsyncConnectionWrapperTransactionManagerWrapper;

        fn establish(database_url: &str) -> diesel::ConnectionResult<Self> {
            let runtime = B::get_runtime();
            let f = C::establish(database_url);
            let inner = runtime.block_on(f)?;
            Ok(Self { inner, runtime })
        }

        fn execute_returning_count<T>(&mut self, source: &T) -> diesel::QueryResult<usize>
        where
            T: diesel::query_builder::QueryFragment<Self::Backend> + diesel::query_builder::QueryId,
        {
            let f = self.inner.execute_returning_count(source);
            self.runtime.block_on(f)
        }

        fn transaction_state(
            &mut self,
        ) -> &mut <Self::TransactionManager as diesel::connection::TransactionManager<Self>>::TransactionStateData{
            self.inner.transaction_state()
        }

        fn instrumentation(&mut self) -> &mut dyn Instrumentation {
            self.inner.instrumentation()
        }

        fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
            self.inner.set_instrumentation(instrumentation);
        }

        fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
            self.inner.set_prepared_statement_cache_size(size)
        }
    }

    impl<C, B> diesel::connection::LoadConnection for AsyncConnectionWrapper<C, B>
    where
        C: crate::AsyncConnection,
        B: BlockOn + Send,
    {
        type Cursor<'conn, 'query>
            = AsyncCursorWrapper<'conn, C::Stream<'conn, 'query>, B>
        where
            Self: 'conn;

        type Row<'conn, 'query>
            = C::Row<'conn, 'query>
        where
            Self: 'conn;

        fn load<'conn, 'query, T>(
            &'conn mut self,
            source: T,
        ) -> diesel::QueryResult<Self::Cursor<'conn, 'query>>
        where
            T: diesel::query_builder::Query
                + diesel::query_builder::QueryFragment<Self::Backend>
                + diesel::query_builder::QueryId
                + 'query,
            Self::Backend: diesel::expression::QueryMetadata<T::SqlType>,
        {
            let f = self.inner.load(source);
            let stream = self.runtime.block_on(f)?;

            Ok(AsyncCursorWrapper {
                stream: Box::pin(stream),
                runtime: &self.runtime,
            })
        }
    }

    pub struct AsyncCursorWrapper<'a, S, B> {
        stream: Pin<Box<S>>,
        runtime: &'a B,
    }

    impl<S, B> Iterator for AsyncCursorWrapper<'_, S, B>
    where
        S: Stream,
        B: BlockOn,
    {
        type Item = S::Item;

        fn next(&mut self) -> Option<Self::Item> {
            let f = self.stream.next();
            self.runtime.block_on(f)
        }
    }

    pub struct AsyncConnectionWrapperTransactionManagerWrapper;

    impl<C, B> diesel::connection::TransactionManager<AsyncConnectionWrapper<C, B>>
        for AsyncConnectionWrapperTransactionManagerWrapper
    where
        C: crate::AsyncConnection,
        B: BlockOn + Send,
    {
        type TransactionStateData =
            <C::TransactionManager as crate::TransactionManager<C>>::TransactionStateData;

        fn begin_transaction(conn: &mut AsyncConnectionWrapper<C, B>) -> diesel::QueryResult<()> {
            let f = <C::TransactionManager as crate::TransactionManager<_>>::begin_transaction(
                &mut conn.inner,
            );
            conn.runtime.block_on(f)
        }

        fn rollback_transaction(
            conn: &mut AsyncConnectionWrapper<C, B>,
        ) -> diesel::QueryResult<()> {
            let f = <C::TransactionManager as crate::TransactionManager<_>>::rollback_transaction(
                &mut conn.inner,
            );
            conn.runtime.block_on(f)
        }

        fn commit_transaction(conn: &mut AsyncConnectionWrapper<C, B>) -> diesel::QueryResult<()> {
            let f = <C::TransactionManager as crate::TransactionManager<_>>::commit_transaction(
                &mut conn.inner,
            );
            conn.runtime.block_on(f)
        }

        fn transaction_manager_status_mut(
            conn: &mut AsyncConnectionWrapper<C, B>,
        ) -> &mut diesel::connection::TransactionManagerStatus {
            <C::TransactionManager as crate::TransactionManager<_>>::transaction_manager_status_mut(
                &mut conn.inner,
            )
        }

        fn is_broken_transaction_manager(conn: &mut AsyncConnectionWrapper<C, B>) -> bool {
            <C::TransactionManager as crate::TransactionManager<_>>::is_broken_transaction_manager(
                &mut conn.inner,
            )
        }
    }

    #[cfg(feature = "r2d2")]
    impl<C, B> diesel::r2d2::R2D2Connection for AsyncConnectionWrapper<C, B>
    where
        B: BlockOn,
        Self: diesel::Connection,
        C: crate::AsyncConnection<Backend = <Self as diesel::Connection>::Backend>
            + crate::pooled_connection::PoolableConnection
            + 'static,
        for<'a> diesel::dsl::select<diesel::dsl::AsExprOf<i32, diesel::sql_types::Integer>>:
            crate::methods::LoadQuery<'a, C, i32>,
        diesel::query_builder::SqlQuery: crate::methods::ExecuteDsl<C>,
    {
        fn ping(&mut self) -> diesel::QueryResult<()> {
            let fut = crate::pooled_connection::PoolableConnection::ping(
                &mut self.inner,
                &crate::pooled_connection::RecyclingMethod::Verified,
            );
            self.runtime.block_on(fut)
        }

        fn is_broken(&mut self) -> bool {
            crate::pooled_connection::PoolableConnection::is_broken(&mut self.inner)
        }
    }

    impl<C, B> diesel::migration::MigrationConnection for AsyncConnectionWrapper<C, B>
    where
        B: BlockOn,
        Self: diesel::Connection,
    {
        fn setup(&mut self) -> diesel::QueryResult<usize> {
            self.batch_execute(diesel::migration::CREATE_MIGRATIONS_TABLE)
                .map(|()| 0)
        }
    }

    #[cfg(feature = "tokio")]
    pub struct Tokio {
        handle: Option<tokio::runtime::Handle>,
        runtime: Option<tokio::runtime::Runtime>,
    }

    #[cfg(feature = "tokio")]
    impl BlockOn for Tokio {
        fn block_on<F>(&self, f: F) -> F::Output
        where
            F: Future,
        {
            if let Some(handle) = &self.handle {
                handle.block_on(f)
            } else if let Some(runtime) = &self.runtime {
                runtime.block_on(f)
            } else {
                unreachable!()
            }
        }

        fn get_runtime() -> Self {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                Self {
                    handle: Some(handle),
                    runtime: None,
                }
            } else {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap();
                Self {
                    handle: None,
                    runtime: Some(runtime),
                }
            }
        }
    }
}
