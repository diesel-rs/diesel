use std::sync::Arc;

use diesel::migration::{Migration, MigrationMetadata, MigrationSource};
use diesel::{Connection, QueryResult};

use crate::MigrationError;

/// A migration source that allows to register rust functions as migrations
///
/// The main use-case of this migration source is to allow writing migrations in Rust
/// instead of in SQL. You need to specify at construction time which for which connection
/// type this migration source is indented to be used with.
/// It allows you to register different kinds of hooks to be executed as migrations later:
///
/// * A closure `Fn(&mut Conn) -> QueryResult<()>` that accepts a mutable connection
///   reference as argument and returns a `QueryResult<()>`. This closure is executed as
///   **up migration**.
/// * A function with a signature of `fn(&mut Conn) -> QueryResult<()>`. This function
///   is executed as **up migration**.
/// * An instance of [`RustMigration`], which allows you to register an up and a down migration
///   as required. This type allows you also to configure the migration behaviour.
///   See the documentation there for details.
/// * Any type implementing [`TypedMigration`]. This allows you to provide a custom type with
///   custom fields to supply additional information to your migration. This trait gives you the
///   full control over how the migration should behave. See the documentation there for details.
///
/// A single migration source can mix all of the variants of migrations listed above.
///
/// # Example
/// ```
/// # include!("../../diesel/src/doctest_setup.rs");
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
/// use diesel::prelude::*;
/// use diesel_migrations::MigrationHarness;
/// use diesel_migrations::RustMigration;
/// use diesel_migrations::RustMigrationSource;
/// use diesel_migrations::TypedMigration;
///
/// # #[cfg(feature = "postgres")]
/// # let connection_url = database_url_from_env("PG_DATABASE_URL");
/// # #[cfg(feature = "sqlite")]
/// # let connection_url = database_url_from_env("SQLITE_DATABASE_URL");
/// # #[cfg(feature = "mysql")]
/// # let connection_url = database_url_from_env("MYSQL_DATABASE_URL");
/// # #[cfg(feature = "postgres")]
/// # type SqliteConnection = PgConnection;
/// # #[cfg(feature = "mysql")]
/// # type SqliteConnection = MysqlConnection;
/// fn migration_function(conn: &mut SqliteConnection) -> QueryResult<()> {
///     diesel::sql_query("SELECT 'EXECUTE YOUR MIGRATION HERE'").execute(conn)?;
///     Ok(())
/// }
///
/// struct CustomMigration(&'static str);
///
/// impl TypedMigration<SqliteConnection> for CustomMigration {
///     fn up(&self, conn: &mut SqliteConnection) -> QueryResult<()> {
/// #       #[cfg(feature = "sqlite")]
///         diesel::sql_query("SELECT 'YOUR MIGRATION', ?")
///             .bind::<diesel::sql_types::Text, _>(self.0)
///             .execute(conn)?;
///         Ok(())
///     }
/// }
///
/// let mut rust_migrations = RustMigrationSource::<SqliteConnection>::new();
///
/// // register migrations callback
/// rust_migrations.add_migration(
///     "2026-01-30-121720_callback",
///     |conn: &mut SqliteConnection| {
///         diesel::sql_query("SELECT 'EXECUTE YOUR MIGRATION HERE'").execute(conn)?;
///         Ok(())
///     },
/// );
///
/// // register a migration as function
/// rust_migrations.add_migration("2026-01-30-122420_function", migration_function);
///
/// // register a RustMigration
/// let migration = RustMigration::new(|conn| {
///     diesel::sql_query("SELECT 'EXECUTE YOUR MIGRATION HERE'").execute(conn)?;
///     Ok(())
/// })
/// .with_down(|conn| {
///     diesel::sql_query("SELECT 'REVERT YOUR MIGRATION HERE'").execute(conn)?;
///     Ok(())
/// });
/// rust_migrations.add_migration("2026-01-30-122520_rust_migration", migration);
///
/// // register a custom migration type
/// rust_migrations.add_migration(
///     "2026-01-30-122920_custom_type",
///     CustomMigration("your custom args"),
/// );
///
/// // run the migrations
/// let mut conn = SqliteConnection::establish(&connection_url)?;
/// conn.run_pending_migrations(rust_migrations)?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct RustMigrationSource<Conn>
where
    Conn: Connection,
{
    migrations: Vec<FunctionBasedMigration<Conn>>,
}

impl<Conn> Clone for RustMigrationSource<Conn>
where
    Conn: Connection,
{
    fn clone(&self) -> Self {
        Self {
            migrations: self.migrations.clone(),
        }
    }
}

impl<Conn> MigrationSource<Conn::Backend> for RustMigrationSource<Conn>
where
    Conn: Connection + 'static,
{
    fn migrations(&self) -> diesel::migration::Result<Vec<Box<dyn Migration<Conn::Backend>>>> {
        Ok(self
            .migrations
            .iter()
            .map(|m| Box::new(m.clone()) as Box<dyn Migration<Conn::Backend>>)
            .collect())
    }
}

impl<Conn> RustMigrationSource<Conn>
where
    Conn: Connection,
{
    /// Create a new empty migration source
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a new migration
    ///
    /// See the documentation on the type itself for examples
    pub fn add_migration(
        &mut self,
        version: impl AsRef<str>,
        migration: impl TypedMigration<Conn> + 'static,
    ) -> Result<&mut Self, MigrationError> {
        self.migrations.push(FunctionBasedMigration {
            migration: Arc::new(migration),
            name: super::file_based_migrations::DieselMigrationName::from_name(version.as_ref())?,
        });
        Ok(self)
    }
}

/// A typed rust migration for a given connection type
///
/// This type describes a typed rust migration for a specific connection type `Conn`
///
/// This trait only requires you to provide an up migration by implementing the relevant function.
/// Optionally you can overwrite the down migration and the migration settings by providing a custom
/// implementation for the relevant functions.
pub trait TypedMigration<Conn> {
    /// The implementation of the up migration.
    ///
    /// This function is supposed to migrate your database from an old schema version
    /// considered valid before this migration was written to a new schema version expected
    /// after this migration was written.
    ///
    /// This will be run inside of a transaction if `Self::run_in_transaction` is
    /// is not customized to return `false`
    fn up(&self, conn: &mut Conn) -> QueryResult<()>;

    /// The implementation of the down migration.
    ///
    /// This function is supposed to revert everything that's done in your up migration.
    ///
    /// The default implementation doesn't perform any action. If you never plan to
    /// revert migrations it can be fine to not provide a custom implementation of
    /// this function.
    ///
    /// This will be run inside of a transaction if `Self::run_in_transaction` is
    /// is not customized to return `false`
    fn down(&self, _conn: &mut Conn) -> QueryResult<()> {
        Ok(())
    }

    /// Should the given migration be run in a transaction or not
    ///
    /// By default diesel runs migrations inside of transactions
    /// (if the underlying database system supports that). This ensures
    /// that each migration is ever only executed as single unit or fails as
    /// single unit.
    ///
    /// Nevertheless specific database operations might require to be run
    /// outside of transactions. If you plan to use such an operation you
    /// want to provide a custom implementation of this function that returns
    /// `false`
    fn run_in_transaction(&self) -> bool {
        true
    }
}

impl<Conn, F> TypedMigration<Conn> for F
where
    F: Fn(&mut Conn) -> QueryResult<()>,
{
    fn up(&self, conn: &mut Conn) -> QueryResult<()> {
        self(conn)
    }
}

type MigrationFunction<Conn> = dyn Fn(&mut Conn) -> QueryResult<()>;

/// A rust side migration
///
/// This type represents a simple rust side migration builder
/// that allows you to register a rust callback as up and down migration
/// and that also allows you to customize the migration settings
///
/// Constructing a `RustMigration` requires to specify an up migration that
/// describes how to migrate your database schema from an old to an new version.
///
/// Down migrations, that describe how to revert the changes done by the up migrations,
/// are optional. If you don't plan to revert migrations you don't need to provide them.
pub struct RustMigration<Conn> {
    up: Box<MigrationFunction<Conn>>,
    down: Option<Box<MigrationFunction<Conn>>>,
    run_in_transaction: bool,
}

impl<Conn> RustMigration<Conn> {
    /// Construct a new instance of this type with a given up migration function.
    ///
    /// This function needs to perform any action to migrate your database from an old version
    /// to the expected new version
    pub fn new(up: impl Fn(&mut Conn) -> QueryResult<()> + 'static) -> Self {
        Self {
            up: Box::new(up),
            down: None,
            run_in_transaction: true,
        }
    }

    /// Register a down migration
    ///
    /// This function allows you to register a down migration to revert any changes done
    /// by the up migration. It is used to restore the database schema used before this migration
    /// was applied in the case of an revert. If you don't plan to revert migrations you don't need to
    /// provide a down migration.
    pub fn with_down(mut self, down: impl Fn(&mut Conn) -> QueryResult<()> + 'static) -> Self {
        self.down = Some(Box::new(down));
        self
    }

    /// Customizes the migration settings to not run this migration in a transaction
    ///
    /// By default diesel will execute migrations inside of a transaction on all database systems
    /// supporting this to ensure that migrations are either fully executed or not.
    ///
    /// Some database operations require to be run outside transactions. If you use such an
    /// operation in either your up or down migration you need to use this function to disable
    /// the default transaction behaviour.
    pub fn without_transaction(mut self) -> Self {
        self.run_in_transaction = false;
        self
    }
}

impl<Conn> TypedMigration<Conn> for RustMigration<Conn> {
    fn up(&self, conn: &mut Conn) -> QueryResult<()> {
        (self.up)(conn)
    }

    fn down(&self, conn: &mut Conn) -> QueryResult<()> {
        if let Some(down) = self.down.as_deref() {
            down(conn)?
        }
        Ok(())
    }

    fn run_in_transaction(&self) -> bool {
        self.run_in_transaction
    }
}

struct FunctionBasedMigration<Conn> {
    migration: Arc<dyn TypedMigration<Conn>>,
    name: super::file_based_migrations::DieselMigrationName,
}

impl<Conn> Clone for FunctionBasedMigration<Conn> {
    fn clone(&self) -> Self {
        Self {
            migration: self.migration.clone(),
            name: self.name.clone(),
        }
    }
}

impl<Conn> Migration<Conn::Backend> for FunctionBasedMigration<Conn>
where
    Conn: Connection + 'static,
    Conn::Backend: 'static,
{
    fn run(
        &self,
        conn: &mut dyn diesel::connection::BoxableConnection<Conn::Backend>,
    ) -> diesel::migration::Result<()> {
        let conn = conn
            .downcast_mut::<Conn>()
            .ok_or("Unable to downcast connection type to the right type")?;
        self.migration.up(conn)?;
        Ok(())
    }

    fn revert(
        &self,
        conn: &mut dyn diesel::connection::BoxableConnection<Conn::Backend>,
    ) -> diesel::migration::Result<()> {
        let conn = conn
            .downcast_mut::<Conn>()
            .ok_or("Unable to downcast connection type to the right type")?;
        self.migration.down(conn)?;
        Ok(())
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        self as &dyn MigrationMetadata
    }

    fn name(&self) -> &dyn diesel::migration::MigrationName {
        &self.name
    }
}

impl<Conn> MigrationMetadata for FunctionBasedMigration<Conn> {
    fn run_in_transaction(&self) -> bool {
        self.migration.run_in_transaction()
    }
}
