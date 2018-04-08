use std::collections::BTreeMap;
use std::iter::FromIterator;
use diesel::deserialize::FromSql;
use diesel::expression::bound::Bound;
use diesel::insertable::ColumnInsertValue;
use diesel::prelude::*;
use diesel::query_builder::{InsertStatement, ValuesClause};
use diesel::query_dsl::methods::ExecuteDsl;
use diesel::sql_types::{VarChar, Timestamp};
use chrono::NaiveDateTime;

use super::schema::__diesel_schema_migrations::dsl::*;

/// A connection which can be passed to the migration methods. This exists only
/// to wrap up some constraints which are meant to hold for *all* connections.
/// This trait will go away at some point in the future. Any Diesel connection
/// should be useable where this trait is required.
pub trait MigrationConnection: Connection {
    fn create_migrations_table_if_needed(&self) -> QueryResult<()>;
    fn recorded_migration_versions(&self) -> QueryResult<BTreeMap<String, NaiveDateTime>>;
    fn last_recorded_migration_version(&self) -> QueryResult<Option<String>>;
    fn record_migration_ran(&self, version: &str) -> QueryResult<()>;
    fn record_migration_reverted(&self, version: &str) -> QueryResult<()>;
}

impl<T> MigrationConnection for T
where
    T: Connection,
    String: FromSql<VarChar, T::Backend>,
    NaiveDateTime: FromSql<Timestamp, T::Backend>,
    // FIXME: HRTB is preventing projecting on any associated types here
    for<'a> InsertStatement<__diesel_schema_migrations, ValuesClause<ColumnInsertValue<version, &'a Bound<VarChar, &'a str>>, __diesel_schema_migrations>>: ExecuteDsl<T>,
{
    fn create_migrations_table_if_needed(&self) -> QueryResult<()> {
        self.execute(
            "CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (\
            version VARCHAR(50) PRIMARY KEY NOT NULL,\
            run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP\
            )",
        )?;
        Ok(())
    }

    fn recorded_migration_versions(&self) -> QueryResult<BTreeMap<String, NaiveDateTime>> {
        __diesel_schema_migrations
            .select((version, run_on))
            .load(self)
            .map(FromIterator::from_iter)
    }

    fn last_recorded_migration_version(&self) -> QueryResult<Option<String>> {
        use diesel::dsl::max;
        __diesel_schema_migrations.select(max(version)).first(self)
    }

    fn record_migration_ran(&self, ver: &str) -> QueryResult<()> {
        try!(
            ::diesel::insert_into(__diesel_schema_migrations)
                .values(&version.eq(ver))
                .execute(self)
        );
        Ok(())
    }

    fn record_migration_reverted(&self, ver: &str) -> QueryResult<()> {
        let target = __diesel_schema_migrations.filter(version.eq(ver));
        try!(
            ::diesel::delete(target)
                .execute(self)
        );
        Ok(())
    }
}
