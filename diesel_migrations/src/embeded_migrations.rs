use std::fmt::Display;

use crate::errors::RunMigrationsError;
use crate::file_based_migrations::{DieselMigrationName, TomlMetadataWrapper};
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationName, MigrationSource, MigrationVersion};

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
    name: EmbeddedName,
    metadata: TomlMetadataWrapper,
}

impl EmbededMigration {
    #[doc(hidden)]
    pub const fn new(
        up: &'static str,
        down: &'static str,
        name: EmbeddedName,
        metadata: TomlMetadataWrapper,
    ) -> Self {
        Self {
            up,
            down,
            name,
            metadata,
        }
    }
}

pub struct EmbeddedName {
    name: &'static str,
}

impl EmbeddedName {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl MigrationName for EmbeddedName {
    fn version(&self) -> MigrationVersion {
        migrations_internals::version_from_string(self.name)
            .expect(
                "This name contains a valid version. We checked this at compile time by our macro",
            )
            .into()
    }
}

impl Display for EmbeddedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'a, DB: Backend> Migration<DB> for &'a EmbededMigration {
    fn run(
        &self,
        conn: &dyn diesel::connection::BoxableConnection<DB>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(conn.batch_execute(self.up).map_err(|e| {
            let name = DieselMigrationName::from_name(self.name.name)
                .expect("We have a vaild name here, we checked this in `embed_migration!`");
            RunMigrationsError::QueryError(name, e)
        })?)
    }

    fn revert(
        &self,
        conn: &dyn diesel::connection::BoxableConnection<DB>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(conn.batch_execute(self.down).map_err(|e| {
            let name = DieselMigrationName::from_name(self.name.name)
                .expect("We have a vaild name here, we checked this in `embed_migration!`");
            RunMigrationsError::QueryError(name, e)
        })?)
    }

    fn metadata(&self) -> &dyn diesel::migration::MigrationMetadata {
        &self.metadata as &dyn diesel::migration::MigrationMetadata
    }

    fn name(&self) -> &dyn MigrationName {
        &self.name
    }
}
