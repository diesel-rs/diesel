use diesel::deserialize::FromSql;
use diesel::expression::bound::Bound;
use diesel::insertable::ColumnInsertValue;
use diesel::prelude::*;
use diesel::query_builder::{InsertStatement, ValuesClause};
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::sql_types::VarChar;
use std::collections::HashSet;
use std::iter::FromIterator;

use super::schema::__diesel_schema_migrations::dsl::*;

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
    // FIXME: HRTB is preventing projecting on any associated types here
    for<'a> InsertStatement<
        __diesel_schema_migrations,
        ValuesClause<
            ColumnInsertValue<version, &'a Bound<VarChar, &'a str>>,
            __diesel_schema_migrations,
        >,
    >: ExecuteDsl<T>,
{
    fn previously_run_migration_versions(&self) -> QueryResult<HashSet<String>> {
        __diesel_schema_migrations
            .select(version)
            .load(self)
            .map(FromIterator::from_iter)
    }

    fn latest_run_migration_version(&self) -> QueryResult<Option<String>> {
        use diesel::dsl::max;
        __diesel_schema_migrations.select(max(version)).first(self)
    }

    fn insert_new_migration(&self, ver: &str) -> QueryResult<()> {
        ::diesel::insert_into(__diesel_schema_migrations)
            .values(&version.eq(ver))
            .execute(self)?;
        Ok(())
    }
}
