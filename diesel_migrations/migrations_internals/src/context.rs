use std::io::{Write, Stdout, stdout};
use std::collections::BTreeMap;
use chrono::NaiveDateTime;

use super::connection::MigrationConnection;
use diesel::migration::{Migration, MigrationSource, MigrationError, RunMigrationsError};


/// A context in which to run migrations
#[derive(Debug)]
pub struct MigrationContext<'a, Conn: 'a, Src, Output> {
    conn: &'a Conn,
    src: Src,
    output: Output
}

/// Iterator over marked migrations
#[derive(Debug)]
pub struct MarkMigrations<I> {
    versions: BTreeMap<String, NaiveDateTime>,
    inner: I
}

pub type MarkedMigrations<M> = MarkMigrations<::std::vec::IntoIter<M>>;

/// Iterator over pending migrations
#[derive(Debug)]
pub struct PendingMigrations<M>(MarkedMigrations<M>);

impl<I> Iterator for MarkMigrations<I> where I: Iterator, I::Item: Migration {
    type Item = (I::Item, Option<NaiveDateTime>);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|migration| {
            let run_on = self.versions.get(migration.version()).cloned();
            (migration, run_on)
        })
    }
}

impl<M> Iterator for PendingMigrations<M> where M: Migration {
    type Item = M;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.0.next() {
                None => None,
                Some((_, Some(_))) => continue,
                Some((m, None)) => Some(m)
            }
        }
    }
}

impl<'a, Conn: MigrationConnection + 'a> MigrationContext<'a, Conn, (), Stdout> {
    /// Create a new migration context without a migration source, which outputs
    /// to stdout.
    pub fn new(conn: &'a Conn) -> Result<Self, RunMigrationsError> {
        conn.create_migrations_table_if_needed()?;
        Ok(MigrationContext { conn, src: (), output: stdout() })
    }
}

impl<'a, Conn: 'a, Src, Output> MigrationContext<'a, Conn, Src, Output> {
    /// Builder method to set the migration source
    pub fn with_source<NewSrc: MigrationSource>(self, src: NewSrc) -> MigrationContext<'a, Conn, NewSrc, Output> {
        MigrationContext { conn: self.conn, src, output: self.output }
    }
    /// Builder method to redirect the migration output
    pub fn with_output<NewOutput: Write>(self, output: NewOutput) -> MigrationContext<'a, Conn, Src, NewOutput> {
        MigrationContext { conn: self.conn, src: self.src, output }
    }
}

// Methods which don't require a `MigrationSource`
impl<'a, Conn: MigrationConnection + 'a, Src, Output: Write> MigrationContext<'a, Conn, Src, Output> {
    /// Run a single migration
    pub fn run_migration(&mut self, migration: &Migration) -> Result<(), RunMigrationsError> {
        if migration.version() != "00000000000000" {
            writeln!(self.output, "Running migration {}", migration.name())?;
        }
        if migration.needs_transaction() {
            self.conn.transaction(|| {
                migration.run(self.conn)?;
                self.conn.record_migration_ran(migration.version())?;
                Ok(())
            })
        } else {
            migration.run(self.conn)?;
            self.conn.transaction(|| {
                self.conn.record_migration_ran(migration.version())?;
                Ok(())
            })
        }
    }
    /// Revert a single migration
    pub fn revert_migration(&mut self, migration: &Migration) -> Result<(), RunMigrationsError> {
        if migration.version() != "00000000000000" {
            writeln!(self.output, "Reverting migration {}", migration.name())?;
        }
        if migration.needs_transaction() {
            self.conn.transaction(|| {
                migration.revert(self.conn)?;
                self.conn.record_migration_reverted(migration.version())?;
                Ok(())
            })
        } else {
            migration.revert(self.conn)?;
            self.conn.transaction(|| {
                self.conn.record_migration_reverted(migration.version())?;
                Ok(())
            })
        }
    }
    /// Mark migrations which have been run
    pub fn mark_migrations<M>(&self, migrations: M) -> Result<MarkMigrations<M::IntoIter>, RunMigrationsError>
        where M: IntoIterator, M::Item: Migration
    {
        Ok(MarkMigrations {
            versions: self.conn.recorded_migration_versions()?,
            inner: migrations.into_iter()
        })
    }
    /// Run multiple migrations in order
    pub fn run_migrations<M>(&mut self, migrations: M) -> Result<(), RunMigrationsError>
        where M: IntoIterator, M::Item: Migration + Sized
    {
        let mut migrations: Vec<_> = migrations.into_iter().collect();
        migrations.sort_by(|a, b| a.version().cmp(b.version()));
        for migration in migrations {
            self.run_migration(&migration)?;
        }
        Ok(())
    }
    /// Revert multiple migrations in reverse order
    pub fn revert_migrations<M>(&mut self, migrations: M) -> Result<(), RunMigrationsError>
        where M: IntoIterator, M::Item: Migration + Sized
    {
        let mut migrations: Vec<_> = migrations.into_iter().collect();
        migrations.sort_by(|a, b| b.version().cmp(a.version()));
        for migration in migrations {
            self.revert_migration(&migration)?;
        }
        Ok(())
    }
}

// Methods which require a `MigrationSource`
impl<'a, Conn: MigrationConnection + 'a, Src: MigrationSource, Output: Write> MigrationContext<'a, Conn, Src, Output> {
    /// Return all the migrations from the migration source, marked with their execution date
    pub fn marked_migrations(&self) -> Result<MarkedMigrations<Src::MigrationEntry>, RunMigrationsError> {
        self.mark_migrations(self.src.list_migrations()?)
    }
    /// Return all pending migrations from the migration source
    pub fn pending_migrations(&self) -> Result<PendingMigrations<Src::MigrationEntry>, RunMigrationsError> {
        self.marked_migrations().map(PendingMigrations)
    }
    /// Run all pending migrations from the migration source
    pub fn run_pending_migrations(&mut self) -> Result<(), RunMigrationsError> {
        let pending_migrations = self.pending_migrations()?;
        self.run_migrations(pending_migrations)
    }
    /// Revert last migration, returns the version that was reverted
    pub fn revert_last_migration(&mut self) -> Result<String, RunMigrationsError> {
        if let Some(last_version) = self.conn.last_recorded_migration_version()? {
            for migration in self.src.list_migrations()? {
                if migration.version() == last_version {
                    self.revert_migration(&migration)?;
                    return Ok(last_version);
                }
            }
            Err(MigrationError::UnknownMigrationVersion(last_version).into())
        } else {
            Err(MigrationError::NoMigrationRun.into())
        }
    }
    /// Check if there are any pending migrations
    pub fn has_pending_migrations(&self) -> Result<bool, RunMigrationsError> {
        Ok(self.pending_migrations()?.next().is_some())
    }
}
