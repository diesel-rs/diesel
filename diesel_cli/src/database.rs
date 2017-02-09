use clap::ArgMatches;
use diesel::expression::sql;
#[cfg(feature="postgres")]
use diesel::pg::PgConnection;
#[cfg(feature="sqlite")]
use diesel::sqlite::SqliteConnection;
#[cfg(feature="mysql")]
use diesel::mysql::MysqlConnection;
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
    #[cfg(feature="mysql")]
    Mysql,
}

impl Backend {
    fn for_url(database_url: &str) -> Self {
        match database_url {
            #[cfg(feature="postgres")]
            _ if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") =>
                Backend::Pg,
            #[cfg(feature="mysql")]
            _ if database_url.starts_with("mysql://") =>
                Backend::Mysql,
            #[cfg(feature="sqlite")]
            _ => Backend::Sqlite,
            #[cfg(not(feature="sqlite"))]
            _ => {
                panic!("{:?} is not a valid database URL. It should start with \
                        `postgres://` or `mysql://`", database_url);
            }
        }
    }
}

pub enum InferConnection {
    #[cfg(feature="postgres")]
    Pg(PgConnection),
    #[cfg(feature="sqlite")]
    Sqlite(SqliteConnection),
    #[cfg(feature="mysql")]
    Mysql(MysqlConnection),
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
            #[cfg(feature="mysql")]
            Backend::Mysql => MysqlConnection::establish(database_url)
                    .map(InferConnection::Mysql),
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
                let (database, postgres_url) = change_database_of_url(database_url, "postgres");
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
        #[cfg(feature="mysql")]
        Backend::Mysql => {
            if MysqlConnection::establish(database_url).is_err() {
                let (database, mysql_url) = change_database_of_url(database_url, "information_schema");
                println!("Creating database: {}", database);
                let conn = try!(MysqlConnection::establish(&mysql_url));
                try!(conn.execute(&format!("CREATE DATABASE {}", database)));
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
            let (database, postgres_url) = change_database_of_url(database_url, "postgres");
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
        #[cfg(feature="mysql")]
        Backend::Mysql => {
            let (database, mysql_url) = change_database_of_url(database_url, "information_schema");
            println!("Dropping database: {}", database);
            let conn = try!(MysqlConnection::establish(&mysql_url));
            try!(conn.execute(&format!("DROP DATABASE IF EXISTS {}", database)));
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
        #[cfg(feature="mysql")]
        InferConnection::Mysql(conn) => {
            select(sql::<Bool>("EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '__diesel_schema_migrations'
                     AND table_schema = DATABASE())"))
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

#[cfg(any(feature="postgres", feature="mysql"))]
fn change_database_of_url(database_url: &str, default_database: &str) -> (String, String) {
    let mut split: Vec<&str> = database_url.split("/").collect();
    let database = split.pop().unwrap();
    let new_url = format!("{}/{}", split.join("/"), default_database);
    (database.to_owned(), new_url)
}

fn handle_error<E: Error, T>(error: E) -> T {
    println!("{}", error);
    ::std::process::exit(1);
}

#[cfg(all(test, any(feature="postgres", feature="mysql")))]
mod tests {
    use super::change_database_of_url;

    #[test]
    fn split_pg_connection_string_returns_postgres_url_and_database() {
        let database = "database".to_owned();
        let base_url = "postgresql://localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!((database, postgres_url), change_database_of_url(&database_url, "postgres"));
    }

    #[test]
    fn split_pg_connection_string_handles_user_and_password() {
        let database = "database".to_owned();
        let base_url = "postgresql://user:password@localhost:5432".to_owned();
        let database_url = format!("{}/{}", base_url, database);
        let postgres_url = format!("{}/{}", base_url, "postgres");
        assert_eq!((database, postgres_url), change_database_of_url(&database_url, "postgres"));
    }
}
