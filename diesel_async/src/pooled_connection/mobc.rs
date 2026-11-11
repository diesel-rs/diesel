//! A pool implementation for `diesel-async` based on [`mobc`]
//!
//! ```rust
//! # include!("../doctest_setup.rs");
//! use diesel::result::Error;
//! use futures_util::FutureExt;
//! use diesel_async::pooled_connection::AsyncDieselConnectionManager;
//! use diesel_async::pooled_connection::mobc::Pool;
//! use diesel_async::{RunQueryDsl, AsyncConnection};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! #     run_test().await.unwrap();
//! # }
//! #
//! # #[cfg(feature = "postgres")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncPgConnection> {
//! #     let db_url = database_url_from_env("PG_DATABASE_URL");
//! let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
//! #     config
//! #  }
//! #
//! # #[cfg(feature = "mysql")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncMysqlConnection> {
//! #     let db_url = database_url_from_env("MYSQL_DATABASE_URL");
//! #    let config = AsyncDieselConnectionManager::<diesel_async::AsyncMysqlConnection>::new(db_url);
//! #     config
//! #  }
//! #
//! # #[cfg(feature = "mariadb")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncMariadbConnection> {
//! #     let db_url = database_url_from_env("MARIADB_DATABASE_URL");
//! #    let config = AsyncDieselConnectionManager::<diesel_async::AsyncMariadbConnection>::new(db_url);
//! #     config
//! #  }
//! #
//! # #[cfg(feature = "sqlite")]
//! # fn get_config() -> AsyncDieselConnectionManager<diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>> {
//! #     let db_url = database_url_from_env("SQLITE_DATABASE_URL");
//! #     let config = AsyncDieselConnectionManager::<diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>>::new(db_url);
//! #     config
//! # }
//! #
//! # async fn run_test() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
//! #     use schema::users::dsl::*;
//! #     let config = get_config();
//! # #[cfg(feature = "postgres")]
//! let pool: Pool<AsyncPgConnection> = Pool::new(config);
//! # #[cfg(not(feature = "postgres"))]
//! # let pool = Pool::new(config);
//! let mut conn = pool.get().await?;
//! # conn.begin_test_transaction();
//! # create_tables(&mut conn).await;
//! # conn.begin_test_transaction();
//! let res = users.load::<(i32, String)>(&mut conn).await?;
//! #     Ok(())
//! # }
//! ```
use super::{AsyncDieselConnectionManager, PoolError, PoolableConnection};
use diesel::query_builder::QueryFragment;
use mobc::Manager;

/// Type alias for using [`mobc::Pool`] with [`diesel-async`]
///
///
/// This is **not** equal to [`mobc::Pool`]. It already uses the correct
/// connection manager and expects only the connection type as generic argument
pub type Pool<C> = mobc::Pool<AsyncDieselConnectionManager<C>>;

/// Type alias for using [`mobc::Connection`] with [`diesel-async`]
pub type PooledConnection<C> = mobc::Connection<AsyncDieselConnectionManager<C>>;

/// Type alias for using [`mobc::Builder`] with [`diesel-async`]
pub type Builder<C> = mobc::Builder<AsyncDieselConnectionManager<C>>;

#[async_trait::async_trait]
impl<C> Manager for AsyncDieselConnectionManager<C>
where
    C: PoolableConnection + 'static,
    for<'a> diesel::dsl::select<diesel::dsl::AsExprOf<i32, diesel::sql_types::Integer>>:
        crate::methods::LoadQuery<'a, C, i32>,
    diesel::query_builder::SqlQuery: QueryFragment<C::Backend>,
{
    type Connection = C;

    type Error = PoolError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        (self.manager_config.custom_setup)(&self.connection_url)
            .await
            .map_err(PoolError::ConnectionError)
    }

    async fn check(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        conn.ping(&self.manager_config.recycling_method)
            .await
            .map_err(PoolError::QueryError)?;
        Ok(conn)
    }

    fn validate(&self, conn: &mut Self::Connection) -> bool {
        std::thread::panicking() || conn.is_broken()
    }
}
