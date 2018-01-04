use clap::ArgMatches;
use diesel::dsl::sql;
use diesel::types::Bool;
use diesel::*;
use migrations_internals as migrations;
#[cfg(any(feature = "postgres", feature = "mysql"))]
use super::query_helper;

use database_error::{DatabaseError, DatabaseResult};

use std::error::Error;
use std::env;
use std::io::stdout;
use std::path::Path;
#[cfg(feature = "postgres")]
use std::fs::{self, File};
#[cfg(feature = "postgres")]
use std::io::Write;

enum Backend {
    #[cfg(feature = "postgres")] Pg,
    #[cfg(feature = "sqlite")] Sqlite,
    #[cfg(feature = "mysql")] Mysql,
}

impl Backend {
    fn for_url(database_url: &str) -> Self {
        match database_url {
            #[cfg(feature = "postgres")]
            _ if database_url.starts_with("postgres://")
                || database_url.starts_with("postgresql://") =>
            {
                Backend::Pg
            }
            #[cfg(feature = "mysql")]
            _ if database_url.starts_with("mysql://") =>
            {
                Backend::Mysql
            }
            #[cfg(feature = "sqlite")]
            _ => Backend::Sqlite,
            #[cfg(not(feature = "sqlite"))]
            _ => {
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
                    "`{}` is not a valid database URL. It should start with {}",
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
    #[cfg(feature = "postgres")] Pg(PgConnection),
    #[cfg(feature = "sqlite")] Sqlite(SqliteConnection),
    #[cfg(feature = "mysql")] Mysql(MysqlConnection),
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
        }.map_err(Into::into)
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
        match ::database::InferConnection::establish(&$database_url).unwrap() {
            #[cfg(feature="postgres")]
            ::database::InferConnection::Pg(ref conn) => $($func)::+ (conn, $($args),*),
            #[cfg(feature="sqlite")]
            ::database::InferConnection::Sqlite(ref conn) => $($func)::+ (conn, $($args),*),
            #[cfg(feature="mysql")]
            ::database::InferConnection::Mysql(ref conn) => $($func)::+ (conn, $($args),*),
        }
    };
}

pub fn reset_database(args: &ArgMatches, migrations_dir: &Path) -> DatabaseResult<()> {
    try!(drop_database(&database_url(args)));
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
        Backend::Pg => if PgConnection::establish(database_url).is_err() {
            let (database, postgres_url) = change_database_of_url(database_url, "postgres");
            println!("Creating database: {}", database);
            let conn = try!(PgConnection::establish(&postgres_url));
            query_helper::create_database(&database).execute(&conn)?;
        },
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => if !::std::path::Path::new(database_url).exists() {
            println!("Creating database: {}", database_url);
            try!(SqliteConnection::establish(database_url));
        },
        #[cfg(feature = "mysql")]
        Backend::Mysql => if MysqlConnection::establish(database_url).is_err() {
            let (database, mysql_url) = change_database_of_url(database_url, "information_schema");
            println!("Creating database: {}", database);
            let conn = try!(MysqlConnection::establish(&mysql_url));
            query_helper::create_database(&database).execute(&conn)?;
        },
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

    #[allow(unreachable_patterns)]
    #[cfg_attr(feature = "clippy", allow(single_match))]
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
        try!(call_with_conn!(database_url, migrations::setup_database()));
        call_with_conn!(
            database_url,
            migrations::run_pending_migrations_in_directory(migrations_dir, &mut stdout())
        ).unwrap_or_else(handle_error);
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
            let conn = try!(PgConnection::establish(&postgres_url));
            if try!(pg_database_exists(&conn, &database)) {
                println!("Dropping database: {}", database);
                query_helper::drop_database(&database)
                    .if_exists()
                    .execute(&conn)?;
            }
        }
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => {
            use std::path::Path;
            use std::fs;

            if Path::new(database_url).exists() {
                println!("Dropping database: {}", database_url);
                try!(fs::remove_file(&database_url));
            }
        }
        #[cfg(feature = "mysql")]
        Backend::Mysql => {
            let (database, mysql_url) = change_database_of_url(database_url, "information_schema");
            let conn = try!(MysqlConnection::establish(&mysql_url));
            if try!(mysql_database_exists(&conn, &database)) {
                println!("Dropping database: {}", database);
                query_helper::drop_database(&database)
                    .if_exists()
                    .execute(&conn)?;
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
fn pg_database_exists(conn: &PgConnection, database_name: &str) -> QueryResult<bool> {
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
fn mysql_database_exists(conn: &MysqlConnection, database_name: &str) -> QueryResult<bool> {
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
        InferConnection::Pg(conn) => select(sql::<Bool>(
            "EXISTS \
             (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = '__diesel_schema_migrations')",
        )).get_result(&conn),
        #[cfg(feature = "sqlite")]
        InferConnection::Sqlite(conn) => select(sql::<Bool>(
            "EXISTS \
             (SELECT 1 \
             FROM sqlite_master \
             WHERE type = 'table' \
             AND name = '__diesel_schema_migrations')",
        )).get_result(&conn),
        #[cfg(feature = "mysql")]
        InferConnection::Mysql(conn) => select(sql::<Bool>(
            "EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '__diesel_schema_migrations'
                     AND table_schema = DATABASE())",
        )).get_result(&conn),
    }.map_err(Into::into)
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
    let mut split: Vec<&str> = database_url.split('/').collect();
    let database = split.pop().unwrap();
    let new_url = format!("{}/{}", split.join("/"), default_database);
    (database.to_owned(), new_url)
}

#[cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
fn handle_error<E: Error, T>(error: E) -> T {
    println!("{}", error);
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
}
