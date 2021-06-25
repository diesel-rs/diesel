#[cfg(any(feature = "postgres", feature = "mysql"))]
use super::query_helper;
use clap::ArgMatches;
use diesel::dsl::sql;
use diesel::sql_types::Bool;
use diesel::*;
use diesel_migrations::FileBasedMigrations;

use crate::database_error::{DatabaseError, DatabaseResult};

use std::env;
use std::error::Error;
#[cfg(feature = "postgres")]
use std::fs::{self, File};
#[cfg(feature = "postgres")]
use std::io::Write;
use std::path::Path;

pub enum Backend {
    #[cfg(feature = "postgres")]
    Pg,
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "mysql")]
    Mysql,
}

impl Backend {
    pub fn for_url(database_url: &str) -> Self {
        match database_url {
            _ if database_url.starts_with("postgres://")
                || database_url.starts_with("postgresql://") =>
            {
                #[cfg(feature = "postgres")]
                {
                    Backend::Pg
                }
                #[cfg(not(feature = "postgres"))]
                {
                    panic!(
                        "Database url `{}` requires the `postgres` feature but it's not enabled.",
                        database_url
                    );
                }
            }
            _ if database_url.starts_with("mysql://") =>
            {
                #[cfg(feature = "mysql")]
                {
                    Backend::Mysql
                }
                #[cfg(not(feature = "mysql"))]
                {
                    panic!(
                        "Database url `{}` requires the `mysql` feature but it's not enabled.",
                        database_url
                    );
                }
            }
            #[cfg(feature = "sqlite")]
            _ => Backend::Sqlite,
            #[cfg(not(feature = "sqlite"))]
            _ => {
                if database_url.starts_with("sqlite://") {
                    panic!(
                        "Database url `{}` requires the `sqlite` feature but it's not enabled.",
                        database_url
                    );
                }

                let mut available_schemes: Vec<&str> = Vec::new();

                // One of these will always be true, or you are compiling
                // diesel_cli without a backend. And why would you ever want to
                // do that?
                if cfg!(feature = "postgres") {
                    available_schemes.push("`postgres://`");
                }
                if cfg!(feature = "mysql") {
                    available_schemes.push("`mysql://`");
                }

                panic!(
                    "`{}` is not a valid database URL. It should start with {}, or maybe you meant to use the `sqlite` feature which is not enabled.",
                    database_url,
                    available_schemes.join(" or ")
                );
            }
            #[cfg(not(any(feature = "mysql", feature = "sqlite", feature = "postgres")))]
            _ => compile_error!(
                "At least one backend must be specified for use with this crate. \
                 You may omit the unneeded dependencies in the following command. \n\n \
                 ex. `cargo install diesel_cli --no-default-features --features mysql postgres sqlite` \n"
            ),
        }
    }
}

pub enum InferConnection {
    #[cfg(feature = "postgres")]
    Pg(PgConnection),
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConnection),
    #[cfg(feature = "mysql")]
    Mysql(MysqlConnection),
}

impl InferConnection {
    pub fn establish(database_url: &str) -> DatabaseResult<Self> {
        match Backend::for_url(database_url) {
            #[cfg(feature = "postgres")]
            Backend::Pg => PgConnection::establish(database_url).map(InferConnection::Pg),
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => {
                SqliteConnection::establish(database_url).map(InferConnection::Sqlite)
            }
            #[cfg(feature = "mysql")]
            Backend::Mysql => MysqlConnection::establish(database_url).map(InferConnection::Mysql),
        }
        .map_err(Into::into)
    }
}

macro_rules! call_with_conn {
    (
        $database_url:expr,
        $($func:ident)::+
    ) => {
        call_with_conn!($database_url, $($func)::+ ())
    };

    (
        $database_url:expr,
        $($func:ident)::+ ($($args:expr),*)
    ) => {
        match crate::database::InferConnection::establish(&$database_url)
            .unwrap_or_else(|err| {crate::database::handle_error_with_database_url(&$database_url, err)})
        {
            #[cfg(feature="postgres")]
            crate::database::InferConnection::Pg(ref mut conn) => $($func)::+ (conn, $($args),*),
            #[cfg(feature="sqlite")]
            crate::database::InferConnection::Sqlite(ref mut conn) => $($func)::+ (conn, $($args),*),
            #[cfg(feature="mysql")]
            crate::database::InferConnection::Mysql(ref mut conn) => $($func)::+ (conn, $($args),*),
        }
    };
}

pub fn reset_database(args: &ArgMatches, migrations_dir: &Path) -> DatabaseResult<()> {
    drop_database(&database_url(args))?;
    setup_database(args, migrations_dir)
}

pub fn setup_database(args: &ArgMatches, migrations_dir: &Path) -> DatabaseResult<()> {
    let database_url = database_url(args);

    create_database_if_needed(&database_url)?;
    create_default_migration_if_needed(&database_url, migrations_dir)?;
    create_schema_table_and_run_migrations_if_needed(&database_url, migrations_dir)?;
    Ok(())
}

pub fn drop_database_command(args: &ArgMatches) -> DatabaseResult<()> {
    drop_database(&database_url(args))
}

/// Creates the database specified in the connection url. It returns an error
/// it was unable to create the database.
fn create_database_if_needed(database_url: &str) -> DatabaseResult<()> {
    match Backend::for_url(database_url) {
        #[cfg(feature = "postgres")]
        Backend::Pg => {
            if PgConnection::establish(database_url).is_err() {
                let (database, postgres_url) = change_database_of_url(database_url, "postgres");
                println!("Creating database: {}", database);
                let mut conn = PgConnection::establish(&postgres_url)?;
                query_helper::create_database(&database).execute(&mut conn)?;
            }
        }
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => {
            if !::std::path::Path::new(database_url).exists() {
                println!("Creating database: {}", database_url);
                SqliteConnection::establish(database_url)?;
            }
        }
        #[cfg(feature = "mysql")]
        Backend::Mysql => {
            if MysqlConnection::establish(database_url).is_err() {
                let (database, mysql_url) =
                    change_database_of_url(database_url, "information_schema");
                println!("Creating database: {}", database);
                let mut conn = MysqlConnection::establish(&mysql_url)?;
                query_helper::create_database(&database).execute(&mut conn)?;
            }
        }
    }

    Ok(())
}

fn create_default_migration_if_needed(
    database_url: &str,
    migrations_dir: &Path,
) -> DatabaseResult<()> {
    let initial_migration_path = migrations_dir.join("00000000000000_diesel_initial_setup");
    if initial_migration_path.exists() {
        return Ok(());
    }

    #[allow(unreachable_patterns, clippy::single_match)]
    match Backend::for_url(database_url) {
        #[cfg(feature = "postgres")]
        Backend::Pg => {
            fs::create_dir_all(&initial_migration_path)?;
            let mut up_sql = File::create(initial_migration_path.join("up.sql"))?;
            up_sql.write_all(include_bytes!("setup_sql/postgres/initial_setup/up.sql"))?;
            let mut down_sql = File::create(initial_migration_path.join("down.sql"))?;
            down_sql.write_all(include_bytes!("setup_sql/postgres/initial_setup/down.sql"))?;
        }
        _ => {} // No default migration for this backend
    }

    Ok(())
}

/// Creates the `__diesel_schema_migrations` table if it doesn't exist. If the
/// table didn't exist, it also runs any pending migrations. Returns a
/// `DatabaseError::ConnectionError` if it can't create the table, and exits
/// with a migration error if it can't run migrations.
fn create_schema_table_and_run_migrations_if_needed(
    database_url: &str,
    migrations_dir: &Path,
) -> DatabaseResult<()> {
    if !schema_table_exists(database_url).unwrap_or_else(handle_error) {
        let migrations =
            FileBasedMigrations::from_path(migrations_dir).unwrap_or_else(handle_error);
        call_with_conn!(database_url, super::run_migrations_with_output(migrations))?;
    };
    Ok(())
}

/// Drops the database specified in the connection url. It returns an error
/// if it was unable to drop the database.
fn drop_database(database_url: &str) -> DatabaseResult<()> {
    match Backend::for_url(database_url) {
        #[cfg(feature = "postgres")]
        Backend::Pg => {
            let (database, postgres_url) = change_database_of_url(database_url, "postgres");
            let mut conn = PgConnection::establish(&postgres_url)?;
            if pg_database_exists(&mut conn, &database)? {
                println!("Dropping database: {}", database);
                query_helper::drop_database(&database)
                    .if_exists()
                    .execute(&mut conn)?;
            }
        }
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => {
            if Path::new(database_url).exists() {
                println!("Dropping database: {}", database_url);
                std::fs::remove_file(&database_url)?;
            }
        }
        #[cfg(feature = "mysql")]
        Backend::Mysql => {
            let (database, mysql_url) = change_database_of_url(database_url, "information_schema");
            let mut conn = MysqlConnection::establish(&mysql_url)?;
            if mysql_database_exists(&mut conn, &database)? {
                println!("Dropping database: {}", database);
                query_helper::drop_database(&database)
                    .if_exists()
                    .execute(&mut conn)?;
            }
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
table! {
    pg_database (datname) {
        datname -> Text,
        datistemplate -> Bool,
    }
}

#[cfg(feature = "postgres")]
fn pg_database_exists(conn: &mut PgConnection, database_name: &str) -> QueryResult<bool> {
    use self::pg_database::dsl::*;

    pg_database
        .select(datname)
        .filter(datname.eq(database_name))
        .filter(datistemplate.eq(false))
        .get_result::<String>(conn)
        .optional()
        .map(|x| x.is_some())
}

#[cfg(feature = "mysql")]
table! {
    information_schema.schemata (schema_name) {
        schema_name -> Text,
    }
}

#[cfg(feature = "mysql")]
fn mysql_database_exists(conn: &mut MysqlConnection, database_name: &str) -> QueryResult<bool> {
    use self::schemata::dsl::*;

    schemata
        .select(schema_name)
        .filter(schema_name.eq(database_name))
        .get_result::<String>(conn)
        .optional()
        .map(|x| x.is_some())
}

/// Returns true if the `__diesel_schema_migrations` table exists in the
/// database we connect to, returns false if it does not.
pub fn schema_table_exists(database_url: &str) -> DatabaseResult<bool> {
    match InferConnection::establish(database_url).unwrap() {
        #[cfg(feature = "postgres")]
        InferConnection::Pg(mut conn) => select(sql::<Bool>(
            "EXISTS \
             (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = '__diesel_schema_migrations')",
        ))
        .get_result(&mut conn),
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(mut conn) => select(sql::<Bool>(
            "EXISTS \
             (SELECT 1 \
             FROM sqlite_master \
             WHERE type = 'table' \
             AND name = '__diesel_schema_migrations')",
        ))
        .get_result(&mut conn),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(mut conn) => select(sql::<Bool>(
            "EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '__diesel_schema_migrations'
                     AND table_schema = DATABASE())",
        ))
        .get_result(&mut conn),
    }
    .map_err(Into::into)
}

pub fn database_url(matches: &ArgMatches) -> String {
    matches
        .value_of("DATABASE_URL")
        .map(|s| s.into())
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| handle_error(DatabaseError::DatabaseUrlMissing))
}

#[cfg(any(feature = "postgres", feature = "mysql"))]
fn change_database_of_url(database_url: &str, default_database: &str) -> (String, String) {
    let base = ::url::Url::parse(database_url).unwrap();
    let database = base.path_segments().unwrap().last().unwrap().to_owned();
    let mut new_url = base.join(default_database).unwrap();
    new_url.set_query(base.query());
    (database, new_url.into())
}

#[allow(clippy::needless_pass_by_value)]
fn handle_error<E: Error, T>(error: E) -> T {
    println!("{}", error);
    ::std::process::exit(1);
}

#[allow(clippy::needless_pass_by_value)]
pub fn handle_error_with_database_url<E: Error, T>(database_url: &str, error: E) -> T {
    eprintln!(
        "Could not connect to database via `{}`: {}",
        database_url, error
    );
    ::std::process::exit(1);
}

#[cfg(all(test, any(feature = "postgres", feature = "mysql")))]
mod tests {
    use super::change_database_of_url;

    #[test]
    fn split_pg_connection_string_returns_postgres_url_and_database() {
        let database = "database".to_owned();
        let base_url = "postgresql://localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!(
            (database, postgres_url),
            change_database_of_url(&database_url, "postgres")
        );
    }

    #[test]
    fn split_pg_connection_string_handles_user_and_password() {
        let database = "database".to_owned();
        let base_url = "postgresql://user:password@localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!(
            (database, postgres_url),
            change_database_of_url(&database_url, "postgres")
        );
    }

    #[test]
    fn split_pg_connection_string_handles_query_string() {
        let database = "database".to_owned();
        let query = "?sslmode=true".to_owned();
        let base_url = "postgresql://user:password@localhost:5432".to_owned();
        let database_url = format!("{}/{}{}", base_url, database, query);
        let postgres_url = format!("{}/{}{}", base_url, "postgres", query);
        assert_eq!(
            (database, postgres_url),
            change_database_of_url(&database_url, "postgres")
        );
    }
}
