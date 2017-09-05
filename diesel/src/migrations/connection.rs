use std::collections::HashSet;
use std::iter::FromIterator;

use prelude::*;
use super::schema::NewMigration;
use super::schema::__diesel_schema_migrations::dsl::*;
use types::{FromSql, VarChar};

/// A connection which can be passed to the migration methods. This exists only
/// to wrap up some constraints which are meant to hold for *all* connections.
/// This trait will go away at some point in the future. Any Diesel connection
/// should be useable where this trait is required.
pub trait MigrationConnection: Connection {
    fn previously_run_migration_versions(&self) -> QueryResult<HashSet<String>>;
    fn latest_run_migration_version(&self) -> QueryResult<Option<String>>;
    fn insert_new_migration(&self, version: &str) -> QueryResult<()>;
}

impl<T> MigrationConnection for T
where
    T: Connection,
    String: FromSql<VarChar, T::Backend>,
    for<'a> &'a NewMigration<'a>: Insertable<__diesel_schema_migrations, T::Backend>,
{
    fn previously_run_migration_versions(&self) -> QueryResult<HashSet<String>> {
        __diesel_schema_migrations
            .select(version)
            .load(self)
            .map(FromIterator::from_iter)
    }

    fn latest_run_migration_version(&self) -> QueryResult<Option<String>> {
        use expression::dsl::max;
        __diesel_schema_migrations.select(max(version)).first(self)
    }

    fn insert_new_migration(&self, ver: &str) -> QueryResult<()> {
        try!(
            ::insert(&NewMigration(ver))
                .into(__diesel_schema_migrations)
                .execute(self)
        );
        Ok(())
    }
}
