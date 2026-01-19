use std::sync::Arc;

use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource};

/// A diesel migration source that combines several other sources
///
/// This source will act like all migrations came from a single source.
/// It orders all the migrations by version
///
/// # Example
/// ```
/// # include!("../../diesel/src/doctest_setup.rs");
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// use diesel::prelude::*;
/// use diesel_migrations::EmbeddedMigrations;
/// use diesel_migrations::CombinedMigrationSource;
/// use crate::diesel_migrations::MigrationHarness;
/// use migrations_macros::embed_migrations;
///
/// pub const PG_MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations/postgres");
/// pub const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations/sqlite");
///
/// # #[cfg(feature = "postgres")]
/// # let connection_url = database_url_from_env("PG_DATABASE_URL");
/// # #[cfg(feature = "sqlite")]
/// # let connection_url = database_url_from_env("SQLITE_DATABASE_URL");
/// # #[cfg(feature = "mysql")]
/// # let connection_url = database_url_from_env("MYSQL_DATABASE_URL");
/// # #[cfg(feature = "postgres")]
/// # type SqliteConnection = PgConnection;
/// # #[cfg(feature = "mysql")]
/// # type SqliteConnection = MysqlConnection;
/// #
/// // Create a new empty combined source
/// let mut combined_sources = CombinedMigrationSource::default();
///
/// // It's not particular meaningful to combine PostgreSQL and SQLite like this,
/// // but that reasonable demonstrates the API
/// combined_sources.add_source(PG_MIGRATIONS);
/// combined_sources.add_source(SQLITE_MIGRATIONS);
///
/// // run the migrations
/// let mut connection = SqliteConnection::establish(&connection_url)?;
/// let res = connection.run_pending_migrations(combined_sources);
/// # assert!(res.is_err(), "This is supposed to fail as you cannot run postgres migrations using sqlite");
/// # Ok(())
/// # }
/// ```
#[derive(Default, Clone)]
pub struct CombinedMigrationSource<DB> {
    migrations: Vec<Arc<dyn MigrationSource<DB> + Send + Sync>>,
}

impl<DB> CombinedMigrationSource<DB>
where
    DB: Backend,
{
    /// Register another source with the given migration source
    pub fn add_source(&mut self, source: impl MigrationSource<DB> + Send + Sync + 'static) {
        self.migrations.push(Arc::new(source))
    }
}

impl<DB> MigrationSource<DB> for CombinedMigrationSource<DB>
where
    DB: Backend,
{
    fn migrations(&self) -> diesel::migration::Result<Vec<Box<dyn Migration<DB>>>> {
        let mut migrations = Vec::new();
        for source in &self.migrations {
            migrations.extend(source.migrations()?);
        }
        migrations.sort_by(|m1, m2| m1.name().version().cmp(&m2.name().version()));
        Ok(migrations)
    }
}
