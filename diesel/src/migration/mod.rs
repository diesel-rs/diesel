//! Representation of migrations

mod errors;
pub use self::errors::{MigrationError, RunMigrationsError};

use connection::{Connection, SimpleConnection};
use result::QueryResult;
use std::path::Path;

/// Represents a migration that interacts with diesel
pub trait Migration {
    /// Get the migration version
    fn version(&self) -> &str;
    /// Apply this migration
    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Revert this migration
    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError>;
    /// Get the migration file path
    fn file_path(&self) -> Option<&Path> {
        None
    }
}

impl Migration for Box<dyn Migration> {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
    }
}

impl<'a> Migration for &'a dyn Migration {
    fn version(&self) -> &str {
        (&**self).version()
    }

    fn run(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).run(conn)
    }

    fn revert(&self, conn: &dyn SimpleConnection) -> Result<(), RunMigrationsError> {
        (&**self).revert(conn)
    }
    fn file_path(&self) -> Option<&Path> {
        (&**self).file_path()
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
    /// # #[macro_use] extern crate diesel;
    /// table! {
    ///      __diesel_schema_migrations(version) {
    ///          version -> Text,
    ///          /// defaults to `CURRENT_TIMESTAMP`
    ///          run_on -> Timestamp,
    ///      }
    /// }
    /// # fn main() {}
    /// ```
    fn setup(&self) -> QueryResult<usize>;
}

#[cfg(feature = "postgres")]
impl MigrationConnection for ::pg::PgConnection {
    fn setup(&self) -> QueryResult<usize> {
        use RunQueryDsl;
        ::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "mysql")]
impl MigrationConnection for ::mysql::MysqlConnection {
    fn setup(&self) -> QueryResult<usize> {
        use RunQueryDsl;
        ::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}

#[cfg(feature = "sqlite")]
impl MigrationConnection for ::sqlite::SqliteConnection {
    fn setup(&self) -> QueryResult<usize> {
        use RunQueryDsl;
        ::sql_query(CREATE_MIGRATIONS_TABLE).execute(self)
    }
}
