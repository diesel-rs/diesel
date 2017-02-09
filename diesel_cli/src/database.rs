use clap::ArgMatches;
use diesel::expression::sql;
#[cfg(feature="postgres")]
use diesel::pg::PgConnection;
#[cfg(feature="sqlite")]
use diesel::sqlite::SqliteConnection;
use diesel::types::Bool;
use diesel::{migrations, Connection, select, LoadDsl};

use database_error::{DatabaseError, DatabaseResult};

use std::error::Error;
use std::env;

enum Backend {
    #[cfg(feature="postgres")]
    Pg,
    #[cfg(feature="sqlite")]
    Sqlite,
}

impl Backend {
    fn for_url(database_url: &str) -> Self {
        match database_url {
            #[cfg(feature="postgres")]
            _ if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") =>
                Backend::Pg,
            #[cfg(feature="sqlite")]
            _ => Backend::Sqlite,
            #[cfg(all(feature="postgres", not(feature="sqlite")))]
            _ => {
                panic!("{:?} is not a valid PostgreSQL URL. It should start with \
                        `postgres://` or `postgresql://`", database_url);
            }
        }
    }
}

pub enum InferConnection {
    #[cfg(feature="postgres")]
    Pg(PgConnection),
    #[cfg(feature="sqlite")]
    Sqlite(SqliteConnection),
}

impl InferConnection {
    pub fn establish(database_url: &str) -> DatabaseResult<Self> {
        match Backend::for_url(database_url) {
            #[cfg(feature="postgres")]
            Backend::Pg => PgConnection::establish(database_url)
                .map(InferConnection::Pg),
            #[cfg(feature="sqlite")]
            Backend::Sqlite => SqliteConnection::establish(database_url)
                    .map(InferConnection::Sqlite),
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
        }
    };
}

pub fn reset_database(args: &ArgMatches) -> DatabaseResult<()> {
    try!(drop_database(&database_url(args)));
    setup_database(args)
}

pub fn setup_database(args: &ArgMatches) -> DatabaseResult<()> {
    let database_url = database_url(args);

    try!(create_database_if_needed(&database_url));
    create_schema_table_and_run_migrations_if_needed(&database_url)
}

pub fn drop_database_command(args: &ArgMatches) -> DatabaseResult<()> {
    drop_database(&database_url(args))
}

/// Creates the database specified in the connection url. It returns an error
/// it it was unable to create the database.
fn create_database_if_needed(database_url: &str) -> DatabaseResult<()> {
    match Backend::for_url(database_url) {
        #[cfg(feature="postgres")]
        Backend::Pg => {
            if PgConnection::establish(database_url).is_err() {
                let (database, postgres_url) = split_pg_connection_string(database_url);
                println!("Creating database: {}", database);
                let conn = try!(PgConnection::establish(&postgres_url));
                try!(conn.execute(&format!("CREATE DATABASE {}", database)));
            }
        },
        #[cfg(feature="sqlite")]
        Backend::Sqlite => {
            if !::std::path::Path::new(database_url).exists() {
                println!("Creating database: {}", database_url);
                try!(SqliteConnection::establish(database_url));
            }
        },
    }

    Ok(())
}

/// Creates the __diesel_schema_migrations table if it doesn't exist. If the
/// table didn't exist, it also runs any pending migrations. Returns a
/// `DatabaseError::ConnectionError` if it can't create the table, and exits
/// with a migration error if it can't run migrations.
fn create_schema_table_and_run_migrations_if_needed(database_url: &str)
    -> DatabaseResult<()>
{
    if !schema_table_exists(database_url).unwrap_or_else(handle_error) {
        try!(call_with_conn!(database_url, migrations::setup_database));
        call_with_conn!(database_url, migrations::run_pending_migrations).unwrap_or_else(handle_error);
    };
    Ok(())
}

/// Drops the database specified in the connection url. It returns an error
/// if it was unable to drop the database.
fn drop_database(database_url: &str) -> DatabaseResult<()> {
    match Backend::for_url(database_url) {
        #[cfg(feature="postgres")]
        Backend::Pg => {
            let (database, postgres_url) = split_pg_connection_string(database_url);
            println!("Dropping database: {}", database);
            let conn = try!(PgConnection::establish(&postgres_url));
            try!(conn.silence_notices(|| {
                conn.execute(&format!("DROP DATABASE IF EXISTS {}", database))
            }));
        },
        #[cfg(feature="sqlite")]
        Backend::Sqlite => {
            println!("Dropping database: {}", database_url);
            try!(::std::fs::remove_file(&database_url));
        },
    }
    Ok(())
}

/// Returns true if the '__diesel_schema_migrations' table exists in the
/// database we connect to, returns false if it does not.
pub fn schema_table_exists(database_url: &str) -> DatabaseResult<bool> {
    match InferConnection::establish(database_url).unwrap() {
        #[cfg(feature="postgres")]
        InferConnection::Pg(conn) => {
            select(sql::<Bool>("EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '__diesel_schema_migrations')"))
                .get_result(&conn)
        },
        #[cfg(feature="sqlite")]
        InferConnection::Sqlite(conn) => {
            select(sql::<Bool>("EXISTS \
                    (SELECT 1 \
                     FROM sqlite_master \
                     WHERE type = 'table' \
                     AND name = '__diesel_schema_migrations')"))
                .get_result(&conn)
        },
    }.map_err(Into::into)
}

pub fn database_url(matches: &ArgMatches) -> String {
    matches.value_of("DATABASE_URL")
        .map(|s| s.into())
        .or(env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| {
            handle_error(DatabaseError::DatabaseUrlMissing)
        })
}

#[cfg(feature="postgres")]
fn split_pg_connection_string(database_url: &str) -> (String, String) {
    let mut split: Vec<&str> = database_url.split("/").collect();
    let database = split.pop().unwrap();
    let postgres_url = format!("{}/{}", split.join("/"), "postgres");
    (database.to_owned(), postgres_url)
}

fn handle_error<E: Error, T>(error: E) -> T {
    println!("{}", error);
    ::std::process::exit(1);
}

#[cfg(all(test, feature="postgres"))]
mod tests {
    use super::split_pg_connection_string;

    #[test]
    fn split_pg_connection_string_returns_postgres_url_and_database() {
        let database = "database".to_owned();
        let base_url = "postgresql://localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!((database, postgres_url), split_pg_connection_string(&database_url));
    }

    #[test]
    fn split_pg_connection_string_handles_user_and_password() {
        let database = "database".to_owned();
        let base_url = "postgresql://user:password@localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!((database, postgres_url), split_pg_connection_string(&database_url));
    }
}
