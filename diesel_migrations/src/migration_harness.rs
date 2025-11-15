use diesel::backend::Backend;
use diesel::migration::{
    Migration, MigrationConnection, MigrationSource, MigrationVersion, Result,
};
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;
use diesel::query_dsl::methods;
use diesel::serialize::ToSql;
use diesel::sql_types::{Text, VarChar};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::time::{Duration, Instant};

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
    ///
    /// # Concurrent Usage Safety
    /// This method can be safely called concurrently from multiple processes. The behavior is as follows:
    ///
    /// * All migrations are applied atomically by the first process that successfully acquires the database lock
    /// * Concurrent processes attempting to run migrations while the lock is held will receive a "database is locked" error
    /// * Processes that start after successful migration completion will find no pending migrations and complete successfully
    /// * Each migration is guaranteed to be applied exactly once
    fn run_pending_migrations<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<Vec<MigrationVersion<'_>>> {
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
    ) -> Result<Vec<MigrationVersion<'_>>> {
        migrations.iter().map(|m| self.run_migration(m)).collect()
    }

    /// Execute the next migration from the given migration source
    fn run_next_migration<S: MigrationSource<DB>>(
        &mut self,
        source: S,
    ) -> Result<MigrationVersion<'_>> {
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
    ) -> Result<Vec<MigrationVersion<'_>>> {
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

impl<C, DB> MigrationHarness<DB> for C
where
    DB: Backend + diesel::internal::migrations::DieselReserveSpecialization,
    C: Connection<Backend = DB> + MigrationConnection + 'static,
    __diesel_schema_migrations::table: methods::BoxedDsl<
        'static,
        DB,
        Output = __diesel_schema_migrations::BoxedQuery<'static, DB>,
    >,
    __diesel_schema_migrations::BoxedQuery<'static, DB, VarChar>:
        methods::LoadQuery<'static, C, MigrationVersion<'static>>,
    diesel::internal::migrations::DefaultValues: QueryFragment<DB>,
    str: ToSql<Text, DB>,
{
    fn run_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        let apply_migration = |conn: &mut C| -> Result<()> {
            migration.run(conn)?;
            diesel::insert_into(__diesel_schema_migrations::table)
                .values(
                    __diesel_schema_migrations::version.eq(migration.name().version().as_owned()),
                )
                .execute(conn)?;
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
            diesel::delete(
                __diesel_schema_migrations::table.find(migration.name().version().as_owned()),
            )
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
            .into_boxed()
            .select(__diesel_schema_migrations::version)
            .order(__diesel_schema_migrations::version.desc())
            .load(self)?)
    }
}

#[cfg(feature = "tracing")]
#[derive(Clone, Copy)]
pub struct TracingOutput(TracingOutputLevel);

#[cfg(feature = "tracing")]
#[derive(Clone, Copy)]
enum TracingOutputLevel {
    Info,
    Debug,
    Trace,
}

#[cfg(feature = "tracing")]
impl TracingOutput {
    fn write(self, line: String) {
        // https://github.com/tokio-rs/tracing/issues/2730
        match self.0 {
            TracingOutputLevel::Info => tracing::info!("{line}"),
            TracingOutputLevel::Debug => tracing::debug!("{line}"),
            TracingOutputLevel::Trace => tracing::trace!("{line}"),
        }
    }
}

/// A migration harness that writes messages
/// into some output for each applied/reverted migration
pub struct HarnessWithOutput<'a, C, W> {
    connection: &'a mut C,
    output: RefCell<W>,
    timed: bool,
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
            timed: false,
        }
    }

    /// Instead of logging before each migration, log after each migration and include the migration's duration
    pub fn timed(mut self) -> Self {
        self.timed = true;
        self
    }
}

impl<'a, C> HarnessWithOutput<'a, C, std::io::Stdout> {
    /// Create a new `HarnessWithOutput` that writes to stdout
    pub fn write_to_stdout<DB>(harness: &'a mut C) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
    {
        Self::new(harness, std::io::stdout())
    }
}

#[cfg(feature = "tracing")]
impl<'a, C> HarnessWithOutput<'a, C, TracingOutput> {
    fn write_to_tracing(harness: &'a mut C, level: TracingOutputLevel) -> Self {
        Self {
            connection: harness,
            output: RefCell::new(TracingOutput(level)),
            timed: false,
        }
    }

    /// Create a new `HarnessWithOutput` that writes to [`tracing::info`]
    pub fn write_to_tracing_info<DB>(harness: &'a mut C) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
    {
        Self::write_to_tracing(harness, TracingOutputLevel::Info)
    }

    /// Create a new `HarnessWithOutput` that writes to [`tracing::debug`]
    pub fn write_to_tracing_debug<DB>(harness: &'a mut C) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
    {
        Self::write_to_tracing(harness, TracingOutputLevel::Debug)
    }

    /// Create a new `HarnessWithOutput` that writes to [`tracing::trace`]
    pub fn write_to_tracing_trace<DB>(harness: &'a mut C) -> Self
    where
        C: MigrationHarness<DB>,
        DB: Backend,
    {
        Self::write_to_tracing(harness, TracingOutputLevel::Trace)
    }
}

impl<C, W, DB> MigrationHarness<DB> for HarnessWithOutput<'_, C, W>
where
    W: Write,
    C: MigrationHarness<DB>,
    DB: Backend,
{
    fn run_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration_is_included_in_output(migration) && !self.timed {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "{}", message_before_run_migration(migration))?;
        }
        let started_at = Instant::now();
        let result = self.connection.run_migration(migration)?;
        if migration_is_included_in_output(migration) && self.timed {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(
                output,
                "{}",
                message_after_run_migration(migration, started_at)?
            )?;
        }
        Ok(result)
    }

    fn revert_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration_is_included_in_output(migration) && !self.timed {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(output, "{}", message_before_revert_migration(migration))?;
        }
        let started_at = Instant::now();
        let result = self.connection.revert_migration(migration)?;
        if migration_is_included_in_output(migration) && self.timed {
            let mut output = self.output.try_borrow_mut()?;
            writeln!(
                output,
                "{}",
                message_after_revert_migration(migration, started_at)?
            )?;
        }
        Ok(result)
    }

    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>> {
        self.connection.applied_migrations()
    }
}

#[cfg(feature = "tracing")]
impl<C, DB> MigrationHarness<DB> for HarnessWithOutput<'_, C, TracingOutput>
where
    C: MigrationHarness<DB>,
    DB: Backend,
{
    fn run_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration_is_included_in_output(migration) && !self.timed {
            let output = self.output.try_borrow()?;
            output.write(message_before_run_migration(migration));
        }
        let started_at = Instant::now();
        let result = self.connection.run_migration(migration)?;
        if migration_is_included_in_output(migration) && self.timed {
            let output = self.output.try_borrow()?;
            output.write(message_after_run_migration(migration, started_at)?);
        }
        Ok(result)
    }

    fn revert_migration(
        &mut self,
        migration: &dyn Migration<DB>,
    ) -> Result<MigrationVersion<'static>> {
        if migration_is_included_in_output(migration) && !self.timed {
            let output = self.output.try_borrow()?;
            output.write(message_before_revert_migration(migration));
        }
        let started_at = Instant::now();
        let result = self.connection.revert_migration(migration)?;
        if migration_is_included_in_output(migration) && self.timed {
            let output = self.output.try_borrow()?;
            output.write(message_after_revert_migration(migration, started_at)?);
        }
        Ok(result)
    }

    fn applied_migrations(&mut self) -> Result<Vec<MigrationVersion<'static>>> {
        self.connection.applied_migrations()
    }
}

fn migration_is_included_in_output(migration: &dyn Migration<impl Backend>) -> bool {
    migration.name().version() != MigrationVersion::from("00000000000000")
}

fn message_before_run_migration(migration: &dyn Migration<impl Backend>) -> String {
    format!("Running migration {}", migration.name())
}

fn message_before_revert_migration(migration: &dyn Migration<impl Backend>) -> String {
    format!("Rolling back migration {}", migration.name())
}

fn message_after_run_migration(
    migration: &dyn Migration<impl Backend>,
    started_at: Instant,
) -> Result<String> {
    // Duration is placed on the left side for alignment
    Ok(format!(
        "(Duration: {}) Ran migration {}",
        display_migration_duration(started_at.elapsed())?,
        migration.name()
    ))
}

fn message_after_revert_migration(
    migration: &dyn Migration<impl Backend>,
    started_at: Instant,
) -> Result<String> {
    // Duration is placed on the left side for alignment
    Ok(format!(
        "(Duration: {}) Rolled back migration {}",
        display_migration_duration(started_at.elapsed())?,
        migration.name()
    ))
}

fn display_migration_duration(duration: Duration) -> Result<String> {
    let total_secs = duration.as_secs();
    let secs = total_secs % 60;
    let total_mins = total_secs / 60;
    let mins = total_mins % 60;
    let total_hours = total_mins / 60;

    let mut result = String::new();

    if total_hours != 0 {
        write!(&mut result, "{total_hours:>2}h")?;
    } else {
        write!(&mut result, "   ")?;
    }

    if total_hours != 0 {
        write!(&mut result, "{mins:02}m")?;
    } else if mins != 0 {
        write!(&mut result, "{mins:>2}m")?;
    } else {
        write!(&mut result, "   ")?;
    }

    if total_mins != 0 {
        write!(&mut result, "{secs:02}s")?;
    } else if secs != 0 {
        write!(&mut result, "{secs:>2}s")?;
    } else {
        write!(&mut result, " {:.0E} s", duration.as_secs_f64())?;
    }

    Ok(result)
}

fn setup_database<Conn: MigrationConnection>(conn: &mut Conn) -> QueryResult<usize> {
    conn.setup()
}

#[cfg(test)]
mod tests {
    use super::display_migration_duration;
    use std::time::Duration;

    #[test]
    fn test_display_migration_duration() {
        for (expected_result, secs) in [
            ("       1E-3 s", 0.00123),
            ("       9E-1 s", 0.9),
            ("       1s", 1.0),
            ("      11s", 11.0),
            ("    1m00s", 60.0),
            ("    1m01s", 61.0),
            ("    1m11s", 71.0),
            ("   11m00s", 660.0),
            (" 1h00m00s", 3600.0),
            ("11h00m00s", 3600.0 * 11.0),
            ("111h00m00s", 3600.0 * 111.0),
        ] {
            assert_eq!(
                display_migration_duration(Duration::from_secs_f64(secs)).unwrap(),
                expected_result
            );
        }
    }
}
