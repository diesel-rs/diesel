#![allow(unused_imports, missing_docs)]
//! Representation of migrations

use crate::backend::Backend;
use crate::connection::{BoxableConnection, Connection};
use crate::deserialize::{FromSql, FromSqlRow};
use crate::expression::AsExpression;
use crate::result::QueryResult;
use crate::serialize::ToSql;
use crate::sql_types::Text;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;
use std::path::Path;

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, FromSqlRow, AsExpression)]
#[sql_type = "Text"]
pub struct MigrationVersion<'a>(Cow<'a, str>);

impl<'a> MigrationVersion<'a> {
    pub fn into_owned(&self) -> MigrationVersion<'static> {
        MigrationVersion(Cow::Owned(self.0.as_ref().to_owned()))
    }
}

impl<'a, DB> FromSql<Text, DB> for MigrationVersion<'a>
where
    String: FromSql<Text, DB>,
    DB: Backend,
{
    fn from_sql(bytes: crate::backend::RawValue<DB>) -> crate::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(Self(Cow::Owned(s)))
    }
}

impl<'a, DB> ToSql<Text, DB> for MigrationVersion<'a>
where
    Cow<'a, str>: ToSql<Text, DB>,
    DB: Backend,
{
    fn to_sql<W: std::io::Write>(
        &self,
        out: &mut crate::serialize::Output<W, DB>,
    ) -> crate::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<'a> From<String> for MigrationVersion<'a> {
    fn from(s: String) -> Self {
        MigrationVersion(Cow::Owned(s))
    }
}

impl<'a> From<&'a str> for MigrationVersion<'a> {
    fn from(s: &'a str) -> Self {
        MigrationVersion(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for MigrationVersion<'a> {
    fn from(s: &'a String) -> Self {
        MigrationVersion(Cow::Borrowed(s))
    }
}

impl<'a> Display for MigrationVersion<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

/// Represents a migration that interacts with diesel
pub trait Migration<DB: Backend> {
    /// Get the migration version
    fn version<'a>(&'a self) -> MigrationVersion<'a>;
    /// Apply this migration
    fn run(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>>;
    /// Revert this migration
    fn revert(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>>;
    /// Get a the attached metadata for this migration
    fn metadata(&self) -> &dyn MigrationMetadata;
}

/// This trait is designed to customize the behaviour
/// of the default migration harness of diesel
///
/// Any new customization option will be added
/// as new function here. Each new function
/// will have a default implementation
/// returning the old a value corresponding
/// to the old uncustomized behaviour
pub trait MigrationMetadata {
    /// Should the current migration be executed in a migration
    /// or not?
    ///
    /// By default this function returns true
    fn run_in_transaction(&self) -> bool {
        true
    }
}

pub trait MigrationSource<DB: Backend> {
    fn migrations(
        &self,
    ) -> Result<Vec<Box<dyn Migration<DB>>>, Box<dyn Error + Send + Sync + 'static>>;
}

impl<'a, DB: Backend> Migration<DB> for Box<dyn Migration<DB> + 'a> {
    fn version<'b>(&'b self) -> MigrationVersion<'b> {
        (&**self).version()
    }

    fn run(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        (&**self).run(conn)
    }

    fn revert(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        (&**self).revert(conn)
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        (&**self).metadata()
    }
}

impl<'a, DB: Backend> Migration<DB> for &'a dyn Migration<DB> {
    fn version<'b>(&'b self) -> MigrationVersion<'b> {
        (&**self).version()
    }

    fn run(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        (&**self).run(conn)
    }

    fn revert(
        &self,
        conn: &dyn BoxableConnection<DB>,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        (&**self).revert(conn)
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        (&**self).metadata()
    }
}

/// Create table statement for the `__diesel_schema_migrations` used
/// used by the postgresql, sqlite and mysql backend
pub const CREATE_MIGRATIONS_TABLE: &str = include_str!("setup_migration_table.sql");

/// A trait indicating that a connection could be used to manage migrations
///
/// Only custom backend implementations need to think about this trait
pub trait MigrationConnection: Connection {
    /// Setup the following table:
    ///
    /// ```rust
    /// diesel::table! {
    ///      __diesel_schema_migrations(version) {
    ///          version -> Text,
    ///          /// defaults to `CURRENT_TIMESTAMP`
    ///          run_on -> Timestamp,
    ///      }
    /// }
    /// ```
    fn setup(&self) -> QueryResult<usize>;
}

#[cfg(feature = "postgres")]
impl MigrationConnection for crate::pg::PgConnection {
    fn setup(&self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "mysql")]
impl MigrationConnection for crate::mysql::MysqlConnection {
    fn setup(&self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "sqlite")]
impl MigrationConnection for crate::sqlite::SqliteConnection {
    fn setup(&self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}
