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

/// A specialized result type representing the result of
/// a migration operation
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// A migration version identifier
///
/// This is used by the migration harness to place migrations
/// in order, therefore two different instances of this type
/// must be sortable
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, FromSqlRow, AsExpression)]
#[diesel(sql_type = Text)]
pub struct MigrationVersion<'a>(Cow<'a, str>);

impl<'a> MigrationVersion<'a> {
    /// Convert the current migration version into
    /// an owned variant with static life time
    pub fn as_owned(&self) -> MigrationVersion<'static> {
        MigrationVersion(Cow::Owned(self.0.as_ref().to_owned()))
    }
}

impl<'a, DB> FromSql<Text, DB> for MigrationVersion<'a>
where
    String: FromSql<Text, DB>,
    DB: Backend,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> crate::deserialize::Result<Self> {
        let s = String::from_sql(bytes)?;
        Ok(Self(Cow::Owned(s)))
    }
}

impl<'a, DB> ToSql<Text, DB> for MigrationVersion<'a>
where
    Cow<'a, str>: ToSql<Text, DB>,
    DB: Backend,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut crate::serialize::Output<'b, '_, DB>,
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

/// Represents the name of a migration
///
/// Users should threat this as `impl Display` type,
/// for implementors of custom migration types
/// this opens the possibility to roll out their own versioning
/// schema.
pub trait MigrationName: Display {
    /// The version corresponding to the current migration name
    fn version(&self) -> MigrationVersion<'_>;
}

/// Represents a migration that interacts with diesel
pub trait Migration<DB: Backend> {
    /// Apply this migration
    fn run(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()>;

    /// Revert this migration
    fn revert(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()>;

    /// Get a the attached metadata for this migration
    fn metadata(&self) -> &dyn MigrationMetadata;

    /// Get the name of the current migration
    ///
    /// The provided name is used by migration harness
    /// to get the version of a migration and to
    /// as something to that is displayed and allows
    /// user to identify a specific migration
    fn name(&self) -> &dyn MigrationName;
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
    /// Whether the current migration is executed in a transaction or not
    ///
    /// By default each migration is executed in a own transaction, but
    /// certain operations (like creating an index on an existing column)
    /// requires running the migration without transaction.
    ///
    /// By default this function returns true
    fn run_in_transaction(&self) -> bool {
        true
    }
}

/// A migration source is an entity that can be used
/// to receive a number of migrations from.
pub trait MigrationSource<DB: Backend> {
    /// Get a list of migrations associated with this
    /// migration source.
    fn migrations(&self) -> Result<Vec<Box<dyn Migration<DB>>>>;
}

impl<'a, DB: Backend> Migration<DB> for Box<dyn Migration<DB> + 'a> {
    fn run(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()> {
        (**self).run(conn)
    }

    fn revert(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()> {
        (**self).revert(conn)
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        (**self).metadata()
    }

    fn name(&self) -> &dyn MigrationName {
        (**self).name()
    }
}

impl<'a, DB: Backend> Migration<DB> for &'a dyn Migration<DB> {
    fn run(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()> {
        (**self).run(conn)
    }

    fn revert(&self, conn: &mut dyn BoxableConnection<DB>) -> Result<()> {
        (**self).revert(conn)
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        (**self).metadata()
    }

    fn name(&self) -> &dyn MigrationName {
        (**self).name()
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
    fn setup(&mut self) -> QueryResult<usize>;
}

#[cfg(feature = "postgres")]
impl MigrationConnection for crate::pg::PgConnection {
    fn setup(&mut self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "mysql")]
impl MigrationConnection for crate::mysql::MysqlConnection {
    fn setup(&mut self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "sqlite")]
impl MigrationConnection for crate::sqlite::SqliteConnection {
    fn setup(&mut self) -> QueryResult<usize> {
        use crate::RunQueryDsl;
        crate::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}
