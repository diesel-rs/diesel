use diesel::migration::{Migration, MigrationVersion, Result};

use crate::async_connection_wrapper::AsyncConnectionWrapper;
use crate::AsyncConnection;

/// A diesel-migration [`MigrationHarness`](diesel_migrations::MigrationHarness) to run migrations
/// via an [`AsyncConnection`](crate::AsyncConnection)
///
/// Internally this harness is using [`tokio::task::block_in_place`] and [`AsyncConnectionWrapper`]
/// to utilize sync Diesel's migration infrastructure. For most applications this shouldn't
/// be problematic as migrations are usually run at application startup and most applications
/// default to use the multithreaded tokio runtime. In turn this also means that you cannot use
/// this migration harness if you use the current thread variant of the tokio runtime or if
/// you run migrations in a very special setup (e.g by using [`tokio::select!`] or [`tokio::join!`]
/// on a future produced by running the migrations). Consider manually construct a blocking task via
/// [`tokio::task::spawn_blocking`] instead.
///
/// ## Example
///
/// ```no_run
/// # include!("doctest_setup.rs");
/// # async fn run_test() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{
/// use diesel_async::AsyncMigrationHarness;
/// use diesel_migrations::{FileBasedMigrations, MigrationHarness};
///
/// let mut connection = connection_no_data().await;
///
/// // Alternativly use `diesel_migrations::embed_migrations!()`
/// // to get a list of migrations
/// let migrations = FileBasedMigrations::find_migrations_directory()?;
///
/// let mut harness = AsyncMigrationHarness::new(connection);
/// harness.run_pending_migrations(migrations)?;
/// // get back the connection from the harness
/// let connection = harness.into_inner();
/// #      Ok(())
/// # }
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// #     run_test().await?;
/// #     Ok(())
/// # }
/// ```
///
/// ## Example with pool
///
/// ```no_run
/// # include!("doctest_setup.rs");
/// # #[cfg(feature = "deadpool")]
/// # use diesel_async::pooled_connection::AsyncDieselConnectionManager;
/// #
/// # #[cfg(all(feature = "postgres", feature = "deadpool"))]
/// # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncPgConnection> {
/// #     let db_url = database_url_from_env("PG_DATABASE_URL");
/// let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_url);
/// #     config
/// #  }
/// #
/// # #[cfg(all(feature = "mysql", feature = "deadpool"))]
/// # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncMysqlConnection> {
/// #     let db_url = database_url_from_env("MYSQL_DATABASE_URL");
/// #    let config = AsyncDieselConnectionManager::<diesel_async::AsyncMysqlConnection>::new(db_url);
/// #     config
/// #  }
/// #
/// # #[cfg(all(feature = "mariadb", feature = "deadpool"))]
/// # fn get_config() -> AsyncDieselConnectionManager<diesel_async::AsyncMariadbConnection> {
/// #     let db_url = database_url_from_env("MARIADB_DATABASE_URL");
/// #    let config = AsyncDieselConnectionManager::<diesel_async::AsyncMariadbConnection>::new(db_url);
/// #     config
/// #  }
/// #
/// # #[cfg(all(feature = "sqlite", feature = "deadpool"))]
/// # fn get_config() -> AsyncDieselConnectionManager<diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>> {
/// #     let db_url = database_url_from_env("SQLITE_DATABASE_URL");
/// #     let config = AsyncDieselConnectionManager::<diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>>::new(db_url);
/// #     config
/// # }
/// # #[cfg(feature = "deadpool")]
/// # async fn run_test() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>{
/// use diesel_async::pooled_connection::deadpool::Pool;
/// use diesel_async::AsyncMigrationHarness;
/// use diesel_migrations::{FileBasedMigrations, MigrationHarness};
///
/// // Alternativly use `diesel_migrations::embed_migrations!()`
/// // to get a list of migrations
/// let migrations = FileBasedMigrations::find_migrations_directory()?;
///
/// let pool = Pool::builder(get_config()).build()?;
/// let mut harness = AsyncMigrationHarness::new(pool.get().await?);
/// harness.run_pending_migrations(migrations)?;
/// #      Ok(())
/// # }
///
/// # #[cfg(not(feature = "deadpool"))]
/// # async fn run_test() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
/// # {
/// #   Ok(())
/// # }
/// #
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// #     run_test().await?;
/// #     Ok(())
/// # }
/// ```
pub struct AsyncMigrationHarness<C> {
    conn: AsyncConnectionWrapper<C, crate::async_connection_wrapper::implementation::Tokio>,
}

impl<C> AsyncMigrationHarness<C>
where
    C: AsyncConnection,
{
    /// Construct a new `AsyncMigrationHarness` from a given connection
    pub fn new(connection: C) -> Self {
        Self {
            conn: AsyncConnectionWrapper::from(connection),
        }
    }

    /// Return the connection stored inside this instance of `AsyncMigrationHarness`
    pub fn into_inner(self) -> C {
        self.conn.into_inner()
    }
}

impl<C> From<C> for AsyncMigrationHarness<C>
where
    C: AsyncConnection,
{
    fn from(value: C) -> Self {
        AsyncMigrationHarness::new(value)
    }
}

impl<C> diesel_migrations::MigrationHarness<C::Backend> for AsyncMigrationHarness<C>
where
    C: AsyncConnection,
    AsyncConnectionWrapper<C, crate::async_connection_wrapper::implementation::Tokio>:
        diesel::Connection<Backend = C::Backend> + diesel_migrations::MigrationHarness<C::Backend>,
{
    fn run_migration(
        &mut self,
        migration: &dyn Migration<C::Backend>,
    ) -> Result<MigrationVersion<'static>> {
        tokio::task::block_in_place(|| {
            diesel_migrations::MigrationHarness::run_migration(&mut self.conn, migration)
        })
    }

    fn revert_migration(
        &mut self,
        migration: &dyn Migration<C::Backend>,
    ) -> Result<MigrationVersion<'static>> {
        tokio::task::block_in_place(|| {
            diesel_migrations::MigrationHarness::revert_migration(&mut self.conn, migration)
        })
    }

    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>> {
        tokio::task::block_in_place(|| {
            diesel_migrations::MigrationHarness::applied_migrations(&mut self.conn)
        })
    }
}
