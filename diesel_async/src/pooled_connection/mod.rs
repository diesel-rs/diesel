//! This module contains support using diesel-async with
//! various async rust connection pooling solutions
//!
//! See the concrete pool implementations for examples:
//! * [deadpool](self::deadpool)
//! * [bb8](self::bb8)
//! * [mobc](self::mobc)
use crate::{AsyncConnection, TransactionManager};
use diesel::QueryResult;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use std::borrow::Cow;
use std::fmt;
use std::future::Future;

#[cfg(feature = "bb8")]
pub mod bb8;
#[cfg(feature = "deadpool")]
pub mod deadpool;
#[cfg(feature = "mobc")]
pub mod mobc;

/// The error used when managing connections with `deadpool`.
#[derive(Debug)]
pub enum PoolError {
    /// An error occurred establishing the connection
    ConnectionError(diesel::result::ConnectionError),

    /// An error occurred pinging the database
    QueryError(diesel::result::Error),
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PoolError::ConnectionError(ref e) => e.fmt(f),
            PoolError::QueryError(ref e) => e.fmt(f),
        }
    }
}

impl std::error::Error for PoolError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PoolError::ConnectionError(connection_error) => Some(connection_error),
            PoolError::QueryError(error) => Some(error),
        }
    }
}

impl From<diesel::result::ConnectionError> for PoolError {
    fn from(value: diesel::result::ConnectionError) -> Self {
        PoolError::ConnectionError(value)
    }
}

impl From<diesel::result::Error> for PoolError {
    fn from(value: diesel::result::Error) -> Self {
        PoolError::QueryError(value)
    }
}

/// Type of the custom setup closure passed to [`ManagerConfig::custom_setup`]
pub type SetupCallback<C> =
    Box<dyn Fn(&str) -> BoxFuture<diesel::ConnectionResult<C>> + Send + Sync>;

/// Type of the recycle check callback for the [`RecyclingMethod::CustomFunction`] variant
pub type RecycleCheckCallback<C> = dyn Fn(&mut C) -> BoxFuture<QueryResult<()>> + Send + Sync;

/// Possible methods of how a connection is recycled.
#[derive(Default)]
pub enum RecyclingMethod<C> {
    /// Only check for open transactions when recycling existing connections
    /// Unless you have special needs this is a safe choice.
    ///
    /// If the database connection is closed you will receive an error on the first place
    /// you actually try to use the connection
    Fast,
    /// In addition to checking for open transactions a test query is executed
    ///
    /// This is slower, but guarantees that the database connection is ready to be used.
    #[default]
    Verified,
    /// Like `Verified` but with a custom query
    CustomQuery(Cow<'static, str>),
    /// Like `Verified` but with a custom callback that allows to perform more checks
    ///
    /// The connection is only recycled if the callback returns `Ok(())`
    CustomFunction(Box<RecycleCheckCallback<C>>),
}

impl<C: fmt::Debug> fmt::Debug for RecyclingMethod<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fast => write!(f, "Fast"),
            Self::Verified => write!(f, "Verified"),
            Self::CustomQuery(arg0) => f.debug_tuple("CustomQuery").field(arg0).finish(),
            Self::CustomFunction(_) => f.debug_tuple("CustomFunction").finish(),
        }
    }
}

/// Configuration object for a Manager.
///
/// This makes it possible to specify which [`RecyclingMethod`]
/// should be used when retrieving existing objects from the `Pool`
/// and it allows to provide a custom setup function.
#[non_exhaustive]
pub struct ManagerConfig<C> {
    /// Method of how a connection is recycled. See [RecyclingMethod].
    pub recycling_method: RecyclingMethod<C>,
    /// Construct a new connection manger
    /// with a custom setup procedure
    ///
    /// This can be used to for example establish a SSL secured
    /// postgres connection
    pub custom_setup: SetupCallback<C>,
}

impl<C> Default for ManagerConfig<C>
where
    C: AsyncConnection + 'static,
{
    fn default() -> Self {
        Self {
            recycling_method: Default::default(),
            custom_setup: Box::new(|url| C::establish(url).boxed()),
        }
    }
}

/// An connection manager for use with diesel-async.
///
/// See the concrete pool implementations for examples:
/// * [deadpool](self::deadpool)
/// * [bb8](self::bb8)
/// * [mobc](self::mobc)
#[allow(dead_code)]
pub struct AsyncDieselConnectionManager<C> {
    connection_url: String,
    manager_config: ManagerConfig<C>,
}

impl<C> fmt::Debug for AsyncDieselConnectionManager<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AsyncDieselConnectionManager<{}>",
            std::any::type_name::<C>()
        )
    }
}

impl<C> AsyncDieselConnectionManager<C>
where
    C: AsyncConnection + 'static,
{
    /// Returns a new connection manager,
    /// which establishes connections to the given database URL.
    #[must_use]
    pub fn new(connection_url: impl Into<String>) -> Self
    where
        C: AsyncConnection + 'static,
    {
        Self::new_with_config(connection_url, Default::default())
    }

    /// Returns a new connection manager,
    /// which establishes connections with the given database URL
    /// and that uses the specified configuration
    #[must_use]
    pub fn new_with_config(
        connection_url: impl Into<String>,
        manager_config: ManagerConfig<C>,
    ) -> Self {
        Self {
            connection_url: connection_url.into(),
            manager_config,
        }
    }
}

#[doc(hidden)]
pub trait PoolableConnection: AsyncConnection {
    /// Check if a connection is still valid
    ///
    /// The default implementation will perform a check based on the provided
    /// recycling method variant
    fn ping(
        &mut self,
        config: &RecyclingMethod<Self>,
    ) -> impl Future<Output = diesel::QueryResult<()>> + Send
    where
        for<'a> Self: 'a,
        for<'a> diesel::dsl::select<diesel::dsl::AsExprOf<i32, diesel::sql_types::Integer>>:
            crate::methods::LoadQuery<'a, Self, i32>,
        diesel::query_builder::SqlQuery: crate::methods::ExecuteDsl<Self>,
    {
        use crate::run_query_dsl::RunQueryDsl;
        use diesel::IntoSql;

        async move {
            match config {
                RecyclingMethod::Fast => Ok(()),
                RecyclingMethod::Verified => {
                    diesel::select(1_i32.into_sql::<diesel::sql_types::Integer>())
                        .get_result::<i32>(self)
                        .await
                        .map(|_| ())
                }
                RecyclingMethod::CustomQuery(query) => diesel::sql_query(query.as_ref())
                    .execute(self)
                    .await
                    .map(|_| ()),
                RecyclingMethod::CustomFunction(c) => c(self).await,
            }
        }
    }

    /// Checks if the connection is broken and should not be reused
    ///
    /// This method should return only contain a fast non-blocking check
    /// if the connection is considered to be broken or not. See
    /// [ManageConnection::has_broken] for details.
    ///
    /// The default implementation uses
    /// [TransactionManager::is_broken_transaction_manager].
    fn is_broken(&mut self) -> bool {
        Self::TransactionManager::is_broken_transaction_manager(self)
    }
}
