use diesel::backend::Backend;
use diesel::dsl;
use diesel::expression::bound::Bound;
use diesel::insertable::ColumnInsertValue;
use diesel::migration::{Migration, MigrationConnection, MigrationSource, MigrationVersion};
use diesel::prelude::*;
use diesel::query_builder::{InsertStatement, ValuesClause};
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::query_dsl::LoadQuery;
use diesel::sql_types::Text;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;

use crate::errors::MigrationError;

diesel::table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

pub trait MigrationHarness<DB: Backend> {
    fn has_pending_migration<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<bool, Box<dyn Error + Send + Sync + 'static>> {
        self.pending_migrations(source).map(|p| !p.is_empty())
    }

    fn run_pending_migrations<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<Vec<MigrationVersion>, Box<dyn Error + Send + Sync + 'static>> {
        self.run_migrations(&self.pending_migrations(source)?)
    }

    fn run_migrations(
        &self,
        migrations: &[Box<dyn Migration<DB>>],
    ) -> Result<Vec<MigrationVersion>, Box<dyn Error + Send + Sync + 'static>> {
        migrations.iter().map(|m| self.run_migration(m)).collect()
    }

    fn revert_all_migrations<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<Vec<MigrationVersion>, Box<dyn Error + Send + Sync + 'static>> {
        let applied_versions = self.applied_migrations()?;
        let mut migrations = source
            .migrations()?
            .into_iter()
            .map(|m| (m.version().into_owned(), m))
            .collect::<HashMap<_, _>>();

        applied_versions
            .into_iter()
            .map(|version| {
                let migration_to_revert = migrations
                    .remove(&version)
                    .ok_or_else(|| MigrationError::UnknownMigrationVersion(version))?;
                self.revert_migration(&migration_to_revert)
            })
            .collect()
    }

    fn run_next_migration<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<MigrationVersion, Box<dyn Error + Send + Sync + 'static>> {
        let pending_migrations = self.pending_migrations(source)?;
        let next_migration = pending_migrations
            .first()
            .ok_or_else(|| MigrationError::NoMigrationRun)?;
        self.run_migration(next_migration)
    }

    fn revert_last_migration<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<MigrationVersion, Box<dyn Error + Send + Sync + 'static>> {
        let applied_versions = self.applied_migrations()?;
        let migrations = source.migrations()?;
        let last_migration_version = applied_versions
            .first()
            .ok_or_else(|| MigrationError::NoMigrationRun)?;
        let migration_to_revert = migrations
            .iter()
            .find(|m| m.version() == *last_migration_version)
            .ok_or_else(|| {
                MigrationError::UnknownMigrationVersion(last_migration_version.into_owned())
            })?;
        self.revert_migration(migration_to_revert)
    }

    fn pending_migrations<S: MigrationSource<DB>>(
        &self,
        source: S,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Box<dyn Error + Send + Sync + 'static>> {
        let applied_versions = self.applied_migrations()?;
        let mut migrations = source
            .migrations()?
            .into_iter()
            .map(|m| (m.version().into_owned(), m))
            .collect::<HashMap<_, _>>();

        for applied_version in applied_versions {
            migrations.remove(&applied_version);
        }

        let mut migrations = migrations.into_iter().map(|(_, m)| m).collect::<Vec<_>>();

        migrations.sort_unstable_by(|a, b| a.version().cmp(&b.version()));

        Ok(migrations)
    }

    fn run_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>>;

    fn revert_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>>;

    fn applied_migrations(
        &self,
    ) -> Result<Vec<MigrationVersion<'static>>, Box<dyn Error + Send + Sync + 'static>>;
}

impl<C, DB> MigrationHarness<DB> for C
where
    DB: Backend,
    C: Connection<Backend = DB> + MigrationConnection + 'static,
    dsl::Order<
        dsl::Select<__diesel_schema_migrations::table, __diesel_schema_migrations::version>,
        dsl::Desc<__diesel_schema_migrations::version>,
    >: LoadQuery<C, MigrationVersion<'static>>,
    for<'a> InsertStatement<
        __diesel_schema_migrations::table,
        ValuesClause<
            ColumnInsertValue<
                __diesel_schema_migrations::version,
                Bound<Text, MigrationVersion<'a>>,
            >,
            __diesel_schema_migrations::table,
        >,
    >: ExecuteDsl<C>,
{
    fn run_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>> {
        let apply_migration = || -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            migration.run(self)?;
            diesel::insert_into(__diesel_schema_migrations::table)
                .values(__diesel_schema_migrations::version.eq(migration.version()))
                .execute(self)?;
            Ok(())
        };

        if migration.metadata().run_in_transaction() {
            self.transaction(apply_migration)?;
        } else {
            apply_migration()?;
        }
        Ok(migration.version().into_owned())
    }

    fn revert_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>> {
        let revert_migration = || -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            migration.revert(self)?;
            diesel::delete(__diesel_schema_migrations::table.find(migration.version()))
                .execute(self)?;
            Ok(())
        };

        if migration.metadata().run_in_transaction() {
            self.transaction(revert_migration)?;
        } else {
            revert_migration()?;
        }
        Ok(migration.version().into_owned())
    }

    fn applied_migrations(
        &self,
    ) -> Result<Vec<MigrationVersion<'static>>, Box<dyn Error + Send + Sync + 'static>> {
        setup_database(self)?;
        Ok(__diesel_schema_migrations::table
            .select(__diesel_schema_migrations::version)
            .order(__diesel_schema_migrations::version.desc())
            .load(self)?)
    }
}

pub struct HarnessWithOutput<'a, C, W> {
    connection: &'a C,
    output: RefCell<W>,
}

impl<'a, C, W> HarnessWithOutput<'a, C, W> {
    pub fn new<DB>(harness: &'a C, output: W) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
        W: Write,
    {
        Self {
            connection: harness,
            output: RefCell::new(output),
        }
    }
}

impl<'a, C> HarnessWithOutput<'a, C, std::io::Stdout> {
    pub fn to_stdout<DB>(harness: &'a C) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
    {
        Self {
            connection: harness,
            output: RefCell::new(std::io::stdout()),
        }
    }
}

impl<'a, C, W, DB> MigrationHarness<DB> for HarnessWithOutput<'a, C, W>
where
    W: Write,
    C: MigrationHarness<DB>,
    DB: Backend,
{
    fn run_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>> {
        if migration.version() != MigrationVersion::from("00000000000000") {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "Running migration {}", migration.version())?;
        }
        self.connection.run_migration(migration)
    }

    fn revert_migration(
        &self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>, Box<dyn Error + Send + Sync + 'static>> {
        if migration.version() != MigrationVersion::from("00000000000000") {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "Rolling backend migration {}", migration.version())?;
        }
        self.connection.revert_migration(migration)
    }

    fn applied_migrations(
        &self,
    ) -> Result<Vec<MigrationVersion<'static>>, Box<dyn Error + Send + Sync + 'static>> {
        self.connection.applied_migrations()
    }
}

fn setup_database<Conn: MigrationConnection>(conn: &Conn) -> QueryResult<usize> {
    conn.setup()
}
