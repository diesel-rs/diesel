//! This module contains a wrapper type
//! that provides a [`crate::AsyncConnection`]
//! implementation for types that implement
//! [`diesel::Connection`]. Using this type
//! might be useful for the following usecases:
//!
//! * using a sync Connection implementation in async context
//! * using the same code base for async crates needing multiple backends
use futures_core::future::BoxFuture;
use std::error::Error;

#[cfg(feature = "sqlite")]
mod sqlite;

/// This is a helper trait that allows to customize the
/// spawning blocking tasks as part of the
/// [`SyncConnectionWrapper`] type. By default a
/// tokio runtime and its spawn_blocking function is used.
pub trait SpawnBlocking {
    /// This function should allow to execute a
    /// given blocking task without blocking the caller
    /// to get the result
    fn spawn_blocking<'a, R>(
        &mut self,
        task: impl FnOnce() -> R + Send + 'static,
    ) -> BoxFuture<'a, Result<R, Box<dyn Error + Send + Sync + 'static>>>
    where
        R: Send + 'static;

    /// This function should be used to construct
    /// a new runtime instance
    fn get_runtime() -> Self;
}

/// A wrapper of a [`diesel::connection::Connection`] usable in async context.
///
/// It implements AsyncConnection if [`diesel::connection::Connection`] fulfils requirements:
/// * it's a [`diesel::connection::LoadConnection`]
/// * its [`diesel::connection::Connection::Backend`] has a [`diesel::query_builder::BindCollector`] implementing [`diesel::query_builder::MoveableBindCollector`]
/// * its [`diesel::connection::LoadConnection::Row`] implements [`diesel::row::IntoOwnedRow`]
///
/// Internally this wrapper type will use `spawn_blocking` on tokio
/// to execute the request on the inner connection. This implies a
/// dependency on tokio and that the runtime is running.
///
/// Note that only SQLite is supported at the moment.
///
/// # Examples
///
/// ```rust
/// # include!("../doctest_setup.rs");
/// use diesel_async::RunQueryDsl;
/// use schema::users;
///
/// async fn some_async_fn() {
/// # let database_url = database_url();
///          use diesel_async::AsyncConnection;
///          use diesel::sqlite::SqliteConnection;
///          let mut conn =
///          SyncConnectionWrapper::<SqliteConnection>::establish(&database_url).await.unwrap();
/// # create_tables(&mut conn).await;
///
///          let all_users = users::table.load::<(i32, String)>(&mut conn).await.unwrap();
/// #         assert_eq!(all_users.len(), 2);
/// }
///
/// # #[cfg(feature = "sqlite")]
/// # #[tokio::main]
/// # async fn main() {
/// #    some_async_fn().await;
/// # }
/// ```
#[cfg(feature = "tokio")]
pub type SyncConnectionWrapper<C, B = self::implementation::Tokio> =
    self::implementation::SyncConnectionWrapper<C, B>;

/// A wrapper of a [`diesel::connection::Connection`] usable in async context.
///
/// It implements AsyncConnection if [`diesel::connection::Connection`] fulfils requirements:
/// * it's a [`diesel::connection::LoadConnection`]
/// * its [`diesel::connection::Connection::Backend`] has a [`diesel::query_builder::BindCollector`] implementing [`diesel::query_builder::MoveableBindCollector`]
/// * its [`diesel::connection::LoadConnection::Row`] implements [`diesel::row::IntoOwnedRow`]
///
/// Internally this wrapper type will use `spawn_blocking` on given type implementing [`SpawnBlocking`] trait
/// to execute the request on the inner connection.
#[cfg(not(feature = "tokio"))]
pub use self::implementation::SyncConnectionWrapper;

pub use self::implementation::SyncTransactionManagerWrapper;

mod implementation {
    use crate::AsyncMultiConnectionHelper;
    use crate::{AsyncConnection, AsyncConnectionCore, SimpleAsyncConnection, TransactionManager};
    use diesel::backend::{Backend, DieselReserveSpecialization};
    use diesel::connection::{CacheSize, Instrumentation, MultiConnectionHelper};
    use diesel::connection::{
        Connection, LoadConnection, TransactionManagerStatus, WithMetadataLookup,
    };
    use diesel::query_builder::bind_collector::BindData;
    use diesel::query_builder::{
        AsQuery, CollectedQuery, MoveableBindCollector, QueryBuilder, QueryFragment, QueryId,
    };
    use diesel::row::IntoOwnedRow;
    use diesel::sql_types::TypeMetadata;
    use diesel::{ConnectionResult, QueryResult};
    use futures_core::stream::BoxStream;
    use futures_util::{FutureExt, StreamExt, TryFutureExt};
    use std::marker::PhantomData;
    use std::sync::{Arc, Mutex};

    use super::*;

    fn from_spawn_blocking_error(
        error: Box<dyn Error + Send + Sync + 'static>,
    ) -> diesel::result::Error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UnableToSendCommand,
            Box::new(error.to_string()),
        )
    }

    pub struct SyncConnectionWrapper<C, S> {
        inner: Arc<Mutex<C>>,
        runtime: S,
    }

    impl<C, S> SimpleAsyncConnection for SyncConnectionWrapper<C, S>
    where
        C: diesel::connection::Connection + 'static,
        S: SpawnBlocking + Send,
    {
        async fn batch_execute(&mut self, query: &str) -> QueryResult<()> {
            let query = query.to_string();
            self.spawn_blocking(move |inner| inner.batch_execute(query.as_str()))
                .await
        }
    }

    impl<C, S, MD, O> AsyncConnectionCore for SyncConnectionWrapper<C, S>
    where
        // Backend bounds
        <C as Connection>::Backend: std::default::Default + DieselReserveSpecialization,
        <C::Backend as Backend>::QueryBuilder: std::default::Default,
        // Connection bounds
        C: Connection + LoadConnection + WithMetadataLookup + 'static,
        <C as Connection>::TransactionManager: Send,
        // BindCollector bounds
        MD: BindData,
        for<'a> <C::Backend as Backend>::BindCollector<'a>:
            MoveableBindCollector<C::Backend, BindData = MD> + std::default::Default,
        // Row bounds
        O: 'static + Send + for<'conn> diesel::row::Row<'conn, C::Backend>,
        for<'conn, 'query> <C as LoadConnection>::Row<'conn, 'query>:
            IntoOwnedRow<'conn, <C as Connection>::Backend, OwnedRow = O>,
        // SpawnBlocking bounds
        S: SpawnBlocking + Send,
    {
        type LoadFuture<'conn, 'query> = BoxFuture<'conn, QueryResult<Self::Stream<'conn, 'query>>>;
        type ExecuteFuture<'conn, 'query> = BoxFuture<'conn, QueryResult<usize>>;
        type Stream<'conn, 'query> = BoxStream<'static, QueryResult<Self::Row<'conn, 'query>>>;
        type Row<'conn, 'query> = O;
        type Backend = <C as Connection>::Backend;

        fn load<'conn, 'query, T>(&'conn mut self, source: T) -> Self::LoadFuture<'conn, 'query>
        where
            T: AsQuery + 'query,
            T::Query: QueryFragment<Self::Backend> + QueryId + 'query,
        {
            self.execute_with_prepared_query(source.as_query(), |conn, query| {
                use diesel::row::IntoOwnedRow;
                let mut cache = <<<C as LoadConnection>::Row<'_, '_> as IntoOwnedRow<
                    <C as Connection>::Backend,
                >>::Cache as Default>::default();
                let cursor = conn.load(&query)?;

                let size_hint = cursor.size_hint();
                let mut out = Vec::with_capacity(size_hint.1.unwrap_or(size_hint.0));
                // we use an explicit loop here to easily propagate possible errors
                // as early as possible
                for row in cursor {
                    out.push(Ok(IntoOwnedRow::into_owned(row?, &mut cache)));
                }

                Ok(out)
            })
            .map_ok(|rows| futures_util::stream::iter(rows).boxed())
            .boxed()
        }

        fn execute_returning_count<'query, T>(
            &mut self,
            source: T,
        ) -> Self::ExecuteFuture<'_, 'query>
        where
            T: QueryFragment<Self::Backend> + QueryId,
        {
            self.execute_with_prepared_query(source, |conn, query| {
                conn.execute_returning_count(&query)
            })
        }
    }

    impl<C, S, MD, O> AsyncConnection for SyncConnectionWrapper<C, S>
    where
        // Backend bounds
        <C as Connection>::Backend: std::default::Default + DieselReserveSpecialization,
        <C::Backend as Backend>::QueryBuilder: std::default::Default,
        // Connection bounds
        C: Connection + LoadConnection + WithMetadataLookup + 'static,
        <C as Connection>::TransactionManager: Send,
        // BindCollector bounds
        MD: BindData,
        for<'a> <C::Backend as Backend>::BindCollector<'a>:
            MoveableBindCollector<C::Backend, BindData = MD> + std::default::Default,
        // Row bounds
        O: 'static + Send + for<'conn> diesel::row::Row<'conn, C::Backend>,
        for<'conn, 'query> <C as LoadConnection>::Row<'conn, 'query>:
            IntoOwnedRow<'conn, <C as Connection>::Backend, OwnedRow = O>,
        // SpawnBlocking bounds
        S: SpawnBlocking + Send,
    {
        type TransactionManager =
            SyncTransactionManagerWrapper<<C as Connection>::TransactionManager>;

        async fn establish(database_url: &str) -> ConnectionResult<Self> {
            let database_url = database_url.to_string();
            let mut runtime = S::get_runtime();

            runtime
                .spawn_blocking(move || C::establish(&database_url))
                .await
                .unwrap_or_else(|e| Err(diesel::ConnectionError::BadConnection(e.to_string())))
                .map(move |c| SyncConnectionWrapper::with_runtime(c, runtime))
        }

        fn transaction_state(
            &mut self,
        ) -> &mut <Self::TransactionManager as TransactionManager<Self>>::TransactionStateData
        {
            self.exclusive_connection().transaction_state()
        }

        fn instrumentation(&mut self) -> &mut dyn Instrumentation {
            // there should be no other pending future when this is called
            // that means there is only one instance of this arc and
            // we can simply access the inner data
            if let Some(inner) = Arc::get_mut(&mut self.inner) {
                inner
                    .get_mut()
                    .unwrap_or_else(|p| p.into_inner())
                    .instrumentation()
            } else {
                panic!("Cannot access shared instrumentation")
            }
        }

        fn set_instrumentation(&mut self, instrumentation: impl Instrumentation) {
            // there should be no other pending future when this is called
            // that means there is only one instance of this arc and
            // we can simply access the inner data
            if let Some(inner) = Arc::get_mut(&mut self.inner) {
                inner
                    .get_mut()
                    .unwrap_or_else(|p| p.into_inner())
                    .set_instrumentation(instrumentation)
            } else {
                panic!("Cannot access shared instrumentation")
            }
        }

        fn set_prepared_statement_cache_size(&mut self, size: CacheSize) {
            // there should be no other pending future when this is called
            // that means there is only one instance of this arc and
            // we can simply access the inner data
            if let Some(inner) = Arc::get_mut(&mut self.inner) {
                inner
                    .get_mut()
                    .unwrap_or_else(|p| p.into_inner())
                    .set_prepared_statement_cache_size(size)
            } else {
                panic!("Cannot access shared cache")
            }
        }
    }

    impl<C, S, MD, O> AsyncMultiConnectionHelper for SyncConnectionWrapper<C, S>
    where
        C: MultiConnectionHelper,
        // Backend bounds
        <C as Connection>::Backend: std::default::Default + DieselReserveSpecialization,
        <C::Backend as Backend>::QueryBuilder: std::default::Default,
        // Connection bounds
        C: Connection + LoadConnection + WithMetadataLookup + 'static,
        <C as Connection>::TransactionManager: Send,
        // BindCollector bounds
        MD: BindData,
        for<'a> <C::Backend as Backend>::BindCollector<'a>:
            MoveableBindCollector<C::Backend, BindData = MD> + std::default::Default,
        // Row bounds
        O: 'static + Send + for<'conn> diesel::row::Row<'conn, C::Backend>,
        for<'conn, 'query> <C as LoadConnection>::Row<'conn, 'query>:
            IntoOwnedRow<'conn, <C as Connection>::Backend, OwnedRow = O>,
        // SpawnBlocking bounds
        S: SpawnBlocking + Send,
    {
        fn to_any<'a>(
            lookup: &mut <Self::Backend as TypeMetadata>::MetadataLookup,
        ) -> &mut (dyn std::any::Any + 'a) {
            C::to_any(lookup)
        }

        fn from_any(
            lookup: &mut dyn std::any::Any,
        ) -> Option<&mut <Self::Backend as diesel::sql_types::TypeMetadata>::MetadataLookup>
        {
            C::from_any(lookup)
        }
    }

    /// A wrapper of a diesel transaction manager usable in async context.
    pub struct SyncTransactionManagerWrapper<T>(PhantomData<T>);

    impl<T, C, S> TransactionManager<SyncConnectionWrapper<C, S>> for SyncTransactionManagerWrapper<T>
    where
        SyncConnectionWrapper<C, S>: AsyncConnection,
        C: Connection + 'static,
        S: SpawnBlocking,
        T: diesel::connection::TransactionManager<C> + Send,
    {
        type TransactionStateData = T::TransactionStateData;

        async fn begin_transaction(conn: &mut SyncConnectionWrapper<C, S>) -> QueryResult<()> {
            conn.spawn_blocking(move |inner| T::begin_transaction(inner))
                .await
        }

        async fn commit_transaction(conn: &mut SyncConnectionWrapper<C, S>) -> QueryResult<()> {
            conn.spawn_blocking(move |inner| T::commit_transaction(inner))
                .await
        }

        async fn rollback_transaction(conn: &mut SyncConnectionWrapper<C, S>) -> QueryResult<()> {
            conn.spawn_blocking(move |inner| T::rollback_transaction(inner))
                .await
        }

        fn transaction_manager_status_mut(
            conn: &mut SyncConnectionWrapper<C, S>,
        ) -> &mut TransactionManagerStatus {
            T::transaction_manager_status_mut(conn.exclusive_connection())
        }
    }

    impl<C, S> SyncConnectionWrapper<C, S> {
        /// Builds a wrapper with this underlying sync connection
        pub fn new(connection: C) -> Self
        where
            C: Connection,
            S: SpawnBlocking,
        {
            SyncConnectionWrapper {
                inner: Arc::new(Mutex::new(connection)),
                runtime: S::get_runtime(),
            }
        }

        /// Builds a wrapper with this underlying sync connection
        /// and runtime for spawning blocking tasks
        pub fn with_runtime(connection: C, runtime: S) -> Self
        where
            C: Connection,
            S: SpawnBlocking,
        {
            SyncConnectionWrapper {
                inner: Arc::new(Mutex::new(connection)),
                runtime,
            }
        }

        /// Run a operation directly with the inner connection
        ///
        /// This function is useful to register custom functions
        /// and collection for Sqlite for example
        ///
        /// # Example
        ///
        /// ```rust
        /// # include!("../doctest_setup.rs");
        /// # #[tokio::main]
        /// # async fn main() {
        /// #     run_test().await.unwrap();
        /// # }
        /// #
        /// # async fn run_test() -> QueryResult<()> {
        /// #     let mut conn = establish_connection().await;
        /// conn.spawn_blocking(|conn| {
        ///    // sqlite.rs sqlite NOCASE only works for ASCII characters,
        ///    // this collation allows handling UTF-8 (barring locale differences)
        ///    conn.register_collation("RUSTNOCASE", |rhs, lhs| {
        ///     rhs.to_lowercase().cmp(&lhs.to_lowercase())
        ///   })
        /// }).await
        ///
        /// # }
        /// ```
        pub fn spawn_blocking<'a, R>(
            &mut self,
            task: impl FnOnce(&mut C) -> QueryResult<R> + Send + 'static,
        ) -> BoxFuture<'a, QueryResult<R>>
        where
            C: Connection + 'static,
            R: Send + 'static,
            S: SpawnBlocking,
        {
            let inner = self.inner.clone();
            self.runtime
                .spawn_blocking(move || {
                    let mut inner = inner.lock().unwrap_or_else(|poison| {
                        // try to be resilient by providing the guard
                        inner.clear_poison();
                        poison.into_inner()
                    });
                    task(&mut inner)
                })
                .unwrap_or_else(|err| QueryResult::Err(from_spawn_blocking_error(err)))
                .boxed()
        }

        fn execute_with_prepared_query<'a, MD, Q, R>(
            &mut self,
            query: Q,
            callback: impl FnOnce(&mut C, &CollectedQuery<MD>) -> QueryResult<R> + Send + 'static,
        ) -> BoxFuture<'a, QueryResult<R>>
        where
            // Backend bounds
            <C as Connection>::Backend: std::default::Default + DieselReserveSpecialization,
            <C::Backend as Backend>::QueryBuilder: std::default::Default,
            // Connection bounds
            C: Connection + LoadConnection + WithMetadataLookup + 'static,
            <C as Connection>::TransactionManager: Send,
            // BindCollector bounds
            MD: BindData,
            for<'b> <C::Backend as Backend>::BindCollector<'b>:
                MoveableBindCollector<C::Backend, BindData = MD> + std::default::Default,
            // Arguments/Return bounds
            Q: QueryFragment<C::Backend> + QueryId,
            R: Send + 'static,
            // SpawnBlocking bounds
            S: SpawnBlocking,
        {
            let backend = C::Backend::default();

            let (collect_bind_result, collector_data) = {
                let exclusive = self.inner.clone();
                let mut inner = exclusive.lock().unwrap_or_else(|poison| {
                    // try to be resilient by providing the guard
                    exclusive.clear_poison();
                    poison.into_inner()
                });
                let mut bind_collector =
                    <<C::Backend as Backend>::BindCollector<'_> as Default>::default();
                let metadata_lookup = inner.metadata_lookup();
                let result = query.collect_binds(&mut bind_collector, metadata_lookup, &backend);
                let collector_data = bind_collector.moveable();

                (result, collector_data)
            };

            let mut query_builder = <<C::Backend as Backend>::QueryBuilder as Default>::default();
            let sql = query
                .to_sql(&mut query_builder, &backend)
                .map(|_| query_builder.finish());
            let is_safe_to_cache_prepared = query.is_safe_to_cache_prepared(&backend);

            self.spawn_blocking(|inner| {
                collect_bind_result?;
                let query = CollectedQuery::new(sql?, is_safe_to_cache_prepared?, collector_data);
                callback(inner, &query)
            })
        }

        /// Gets an exclusive access to the underlying diesel Connection
        ///
        /// It panics in case of shared access.
        /// This is typically used only used during transaction.
        pub(self) fn exclusive_connection(&mut self) -> &mut C
        where
            C: Connection,
        {
            // there should be no other pending future when this is called
            // that means there is only one instance of this Arc and
            // we can simply access the inner data
            if let Some(conn_mutex) = Arc::get_mut(&mut self.inner) {
                conn_mutex
                    .get_mut()
                    .expect("Mutex is poisoned, a thread must have panicked holding it.")
            } else {
                panic!("Cannot access shared transaction state")
            }
        }
    }

    #[cfg(any(
        feature = "deadpool",
        feature = "bb8",
        feature = "mobc",
        feature = "r2d2"
    ))]
    impl<C, S> crate::pooled_connection::PoolableConnection for SyncConnectionWrapper<C, S>
    where
        Self: AsyncConnection,
    {
        fn is_broken(&mut self) -> bool {
            Self::TransactionManager::is_broken_transaction_manager(self)
        }
    }

    #[cfg(feature = "tokio")]
    pub enum Tokio {
        Handle(tokio::runtime::Handle),
        Runtime(tokio::runtime::Runtime),
    }

    #[cfg(feature = "tokio")]
    impl SpawnBlocking for Tokio {
        fn spawn_blocking<'a, R>(
            &mut self,
            task: impl FnOnce() -> R + Send + 'static,
        ) -> BoxFuture<'a, Result<R, Box<dyn Error + Send + Sync + 'static>>>
        where
            R: Send + 'static,
        {
            let fut = match self {
                Tokio::Handle(handle) => CancelWrapper {
                    future: handle.spawn_blocking(task),
                    handle: handle.clone(),
                },
                Tokio::Runtime(runtime) => CancelWrapper {
                    future: runtime.spawn_blocking(task),
                    handle: runtime.handle().clone(),
                },
            };

            fut.map_err(Box::from).boxed()
        }

        fn get_runtime() -> Self {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                Tokio::Handle(handle)
            } else {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();

                Tokio::Runtime(runtime)
            }
        }
    }

    #[cfg(feature = "tokio")]
    use std::pin::Pin;

    #[cfg(feature = "tokio")]
    /// Wrapper around a task that makes sure it finishes before the Future gets cancelled
    struct CancelWrapper<R> {
        handle: tokio::runtime::Handle,
        future: tokio::task::JoinHandle<R>,
    }

    #[cfg(feature = "tokio")]
    impl<R> std::future::Future for CancelWrapper<R> {
        type Output = Result<R, tokio::task::JoinError>;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            let future = Pin::new(&mut self.get_mut().future);
            future.poll(cx)
        }
    }

    #[cfg(feature = "tokio")]
    impl<R> Drop for CancelWrapper<R> {
        fn drop(&mut self) {
            let future = Pin::new(&mut self.future);
            if !future.is_finished() {
                tokio::task::block_in_place(|| {
                    // we don't care about the result, just make sure the task is finished before continuing
                    let _ = self.handle.block_on(future);
                });
            }
        }
    }
}
