use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::dsl;
use diesel::migration::{
    Migration, MigrationConnection, MigrationSource, MigrationVersion, Result,
};
use diesel::prelude::*;
use diesel::query_builder::{DeleteStatement, InsertStatement, IntoUpdateTarget};
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::query_dsl::LoadQuery;
use diesel::serialize::ToSql;
use diesel::sql_types::Text;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;

use crate::errors::MigrationError;

diesel::table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

/// A migration harness is an entity which applies migration to an existing database
pub trait MigrationHarness<DB: Backend> {
    /// Checks if the database represented by the current harness has unapplied migrations
    fn has_pending_migration<S: MigrationSource<DB>>(&mut self, source: S) -> Result<bool> {
        self.pending_migrations(source).map(|p| !p.is_empty())
    }

    /// Execute all unapplied migrations for a given migration source
    fn run_pending_migrations<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<Vec<MigrationVersion>> {
        let pending = self.pending_migrations(source)?;
        self.run_migrations(&pending)
    }

    /// Execute all migrations in the given list
    ///
    /// This method does not check if a certain migration was already applied or not
    #[doc(hidden)]
    fn run_migrations(
        &mut self,
        migrations: &[Box<dyn Migration<DB>>],
    ) -> Result<Vec<MigrationVersion>> {
        migrations.iter().map(|m| self.run_migration(m)).collect()
    }

    /// Execute the next migration from the given migration source
    fn run_next_migration<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<MigrationVersion> {
        let pending_migrations = self.pending_migrations(source)?;
        let next_migration = pending_migrations
            .first()
            .ok_or(MigrationError::NoMigrationRun)?;
        self.run_migration(next_migration)
    }

    /// Revert all applied migrations from a given migration source
    fn revert_all_migrations<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<Vec<MigrationVersion>> {
        let applied_versions = self.applied_migrations()?;
        let mut migrations = source
            .migrations()?
            .into_iter()
            .map(|m| (m.name().version().as_owned(), m))
            .collect::<HashMap<_, _>>();

        applied_versions
            .into_iter()
            .map(|version| {
                let migration_to_revert = migrations
                    .remove(&version)
                    .ok_or(MigrationError::UnknownMigrationVersion(version))?;
                self.revert_migration(&migration_to_revert)
            })
            .collect()
    }

    /// Revert the last migration from a given migration source
    ///
    /// This method returns a error if the given migration source does not
    /// contain the last applied migration
    fn revert_last_migration<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<MigrationVersion<'static>> {
        let applied_versions = self.applied_migrations()?;
        let migrations = source.migrations()?;
        let last_migration_version = applied_versions
            .first()
            .ok_or(MigrationError::NoMigrationRun)?;
        let migration_to_revert = migrations
            .iter()
            .find(|m| m.name().version() == *last_migration_version)
            .ok_or_else(|| {
                MigrationError::UnknownMigrationVersion(last_migration_version.as_owned())
            })?;
        self.revert_migration(migration_to_revert)
    }

    /// Get a list of non applied migrations for a specific migration source
    ///
    /// The returned migration list is sorted in ascending order by the individual version
    /// of each migration
    fn pending_migrations<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<Vec<Box<dyn Migration<DB>>>> {
        let applied_versions = self.applied_migrations()?;
        let mut migrations = source
            .migrations()?
            .into_iter()
            .map(|m| (m.name().version().as_owned(), m))
            .collect::<HashMap<_, _>>();

        for applied_version in applied_versions {
            migrations.remove(&applied_version);
        }

        let mut migrations = migrations.into_values().collect::<Vec<_>>();

        migrations.sort_unstable_by(|a, b| a.name().version().cmp(&b.name().version()));

        Ok(migrations)
    }

    /// Apply a single migration
    ///
    /// Types implementing this trait should call [`Migration::run`] internally and record
    /// that a specific migration version was executed afterwards.
    fn run_migration(&mut self, migration: &dyn Migration<DB>)
        -> Result<MigrationVersion<'static>>;

    /// Revert a single migration
    ///
    /// Types implementing this trait should call [`Migration::revert`] internally
    /// and record that a specific migration version was reverted afterwards.
    fn revert_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>>;

    /// Get a list of already applied migration versions
    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>>;
}

impl<'b, C, DB> MigrationHarness<DB> for C
where
    DB: Backend,
    C: Connection<Backend = DB> + MigrationConnection + 'static,
    dsl::Order<
        dsl::Select<__diesel_schema_migrations::table, __diesel_schema_migrations::version>,
        dsl::Desc<__diesel_schema_migrations::version>,
    >: LoadQuery<'b, C, MigrationVersion<'static>>,
    for<'a> InsertStatement<
        __diesel_schema_migrations::table,
        <dsl::Eq<__diesel_schema_migrations::version, MigrationVersion<'static>> as Insertable<
            __diesel_schema_migrations::table,
        >>::Values,
    >: diesel::query_builder::QueryFragment<DB> + ExecuteDsl<C, DB>,
    DeleteStatement<
        <dsl::Find<
            __diesel_schema_migrations::table,
            MigrationVersion<'static>,
        > as HasTable>::Table,
        <dsl::Find<
            __diesel_schema_migrations::table,
            MigrationVersion<'static>,
        > as IntoUpdateTarget>::WhereClause,
    >: ExecuteDsl<C>,
    str: ToSql<Text, DB>,
{
    fn run_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        let apply_migration = |conn: &mut C| -> Result<()> {
            migration.run(conn)?;
            diesel::insert_into(__diesel_schema_migrations::table)
                .values(__diesel_schema_migrations::version.eq(migration.name().version().as_owned())).execute(conn)?;
            Ok(())
        };

        if migration.metadata().run_in_transaction() {
            self.transaction(apply_migration)?;
        } else {
            apply_migration(self)?;
        }
        Ok(migration.name().version().as_owned())
    }

    fn revert_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        let revert_migration = |conn: &mut C| -> Result<()> {
            migration.revert(conn)?;
            diesel::delete(__diesel_schema_migrations::table.find(migration.name().version().as_owned()))
               .execute(conn)?;
            Ok(())
        };

        if migration.metadata().run_in_transaction() {
            self.transaction(revert_migration)?;
        } else {
            revert_migration(self)?;
        }
        Ok(migration.name().version().as_owned())
    }

    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>> {
        setup_database(self)?;
        Ok(__diesel_schema_migrations::table
            .select(__diesel_schema_migrations::version)
            .order(__diesel_schema_migrations::version.desc())
            .load(self)?)
    }
}

/// A migration harness that writes messages
/// into some output for each applied/reverted migration
pub struct HarnessWithOutput<'a, C, W> {
    connection: &'a mut C,
    output: RefCell<W>,
}

impl<'a, C, W> HarnessWithOutput<'a, C, W> {
    /// Create a new `HarnessWithOutput` that writes to a specific writer
    pub fn new<DB>(harness: &'a mut C, output: W) -> Self
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
    /// Create a new `HarnessWithOutput` that writes to stdout
    pub fn write_to_stdout<DB>(harness: &'a mut C) -> Self
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
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration.name().version() != MigrationVersion::from("00000000000000") {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "Running migration {}", migration.name())?;
        }
        self.connection.run_migration(migration)
    }

    fn revert_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration.name().version() != MigrationVersion::from("00000000000000") {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "Rolling back migration {}", migration.name())?;
        }
        self.connection.revert_migration(migration)
    }

    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>> {
        self.connection.applied_migrations()
    }
}

fn setup_database<Conn: MigrationConnection>(conn: &mut Conn) -> QueryResult<usize> {
    conn.setup()
}
