use crate::errors::RunMigrationsError;
use crate::file_based_migrations::TomlMetadataWrapper;
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource, MigrationVersion};

pub struct EmbededMigrations {
    migrations: &'static [EmbededMigration],
}

impl EmbededMigrations {
    #[doc(hidden)]
    pub const fn new(migrations: &'static [EmbededMigration]) -> Self {
        Self { migrations }
    }
}

impl<DB: Backend> MigrationSource<DB> for EmbededMigrations {
    fn migrations(
        &self,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Box<dyn std::error::Error + Send + Sync + 'static>>
    {
        Ok(self
            .migrations
            .iter()
            .map(|m| Box::new(m) as Box<dyn Migration<DB>>)
            .collect())
    }
}

#[doc(hidden)]
pub struct EmbededMigration {
    up: &'static str,
    down: &'static str,
    version: &'static str,
    metadata: TomlMetadataWrapper,
}

impl EmbededMigration {
    #[doc(hidden)]
    pub const fn new(
        up: &'static str,
        down: &'static str,
        version: &'static str,
        metadata: TomlMetadataWrapper,
    ) -> Self {
        Self {
            up,
            down,
            version,
            metadata,
        }
    }
}

impl<'a, DB: Backend> Migration<DB> for &'a EmbededMigration {
    fn version<'b>(&'b self) -> MigrationVersion<'b> {
        self.version.into()
    }

    fn run(
        &self,
        conn: &dyn diesel::connection::BoxableConnection<DB>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(conn.batch_execute(self.up).map_err(|e| {
            RunMigrationsError::QueryError(Migration::<DB>::version(self).into_owned(), e)
        })?)
    }

    fn revert(
        &self,
        conn: &dyn diesel::connection::BoxableConnection<DB>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(conn.batch_execute(self.down).map_err(|e| {
            RunMigrationsError::QueryError(Migration::<DB>::version(self).into_owned(), e)
        })?)
    }

    fn metadata(&self) -> &dyn diesel::migration::MigrationMetadata {
        &self.metadata as &dyn diesel::migration::MigrationMetadata
    }
}
