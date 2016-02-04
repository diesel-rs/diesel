extern crate chrono;
extern crate clap;
extern crate diesel;
extern crate dotenv;

mod database_error;

use chrono::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use diesel::expression::sql;
use diesel::types::Bool;
use diesel::pg::PgConnection;
use diesel::sqlite::SqliteConnection;
use diesel::migrations::schema::*;
use diesel::types::{FromSql, VarChar};
use diesel::{migrations, Connection, select, LoadDsl, Insertable};
use std::error::Error;
use std::io::stdout;
use std::path::{PathBuf, Path};
use std::{env, fs};

use self::database_error::{DatabaseError, DatabaseResult};

macro_rules! call_with_conn {
    ( $database_url:ident,
      $func:path
    ) => {{
        match backend(&$database_url) {
            "postgres" => {
                let conn = PgConnection::establish(&$database_url).unwrap();
                $func(&conn)
            },
            "sqlite" => {
                let conn = SqliteConnection::establish(&$database_url).unwrap();
                $func(&conn)
            },
            _ => unreachable!("The backend function should ensure we never get here."),
        }
    }};
}

fn main() {
    let database_arg = || Arg::with_name("DATABASE_URL")
        .long("database-url")
        .help("Specifies the database URL to connect to. Falls back to \
                   the DATABASE_URL environment variable if unspecified.")
        .takes_value(true);

    let migration_subcommand = SubCommand::with_name("migration")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs all pending migrations")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("revert")
                .about("Reverts the latest run migration")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("redo")
                .about("Reverts and re-runs the latest migration. Useful \
                      for testing that a migration can in fact be reverted.")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("generate")
                .about("Generate a new migration with the given name, and \
                      the current timestamp as the version")
                .arg(Arg::with_name("MIGRATION_NAME")
                     .help("The name of the migration to create")
                     .required(true)
                 )
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let setup_subcommand = SubCommand::with_name("setup")
        .about("Creates the migrations directory, creates the database \
                specified in your DATABASE_URL, and runs existing migrations.")
        .arg(database_arg());

    let database_subcommand = SubCommand::with_name("database")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("setup")
                .about("Creates the migrations directory, creates the database \
                        specified in your DATABASE_URL, and runs existing migrations.")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("reset")
                .about("Resets your database by dropping the database specified \
                        in your DATABASE_URL and then running `diesel database setup`.")
                .arg(database_arg())
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(migration_subcommand)
        .subcommand(setup_subcommand)
        .subcommand(database_subcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches),
        ("setup", Some(matches)) => run_setup_command(matches),
        ("database", Some(matches)) => run_database_command(matches),
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn run_migration_command(matches: &ArgMatches) {
    let database_url = database_url(matches);

    match matches.subcommand() {
        ("run", Some(_)) => {
            call_with_conn!(database_url, migrations::run_pending_migrations)
                .map_err(handle_error).unwrap();
        }
        ("revert", Some(_)) => {
            call_with_conn!(database_url, migrations::revert_latest_migration)
                .map_err(handle_error).unwrap();
        }
        ("redo", Some(_)) => {
            call_with_conn!(database_url, redo_latest_migration);
        }
        ("generate", Some(args)) => {
            let migration_name = args.value_of("MIGRATION_NAME").unwrap();
            let timestamp = Local::now().format("%Y%m%d%H%M%S");
            let versioned_name = format!("{}_{}", &timestamp, migration_name);
            let mut migration_dir = migrations::find_migrations_directory()
                .map_err(handle_error).unwrap().join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            let migration_dir_relative = convert_absolute_path_to_relative(
                &mut migration_dir,
                &mut env::current_dir().unwrap()
            );

            let up_path = migration_dir.join("up.sql");
            println!("Creating {}", migration_dir_relative.join("up.sql").display());
            fs::File::create(up_path).unwrap();
            let down_path = migration_dir.join("down.sql");
            println!("Creating {}", migration_dir_relative.join("down.sql").display());
            fs::File::create(down_path).unwrap();
        }
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn run_setup_command(matches: &ArgMatches) {
    migrations::find_migrations_directory()
        .unwrap_or_else(|_|
                        create_migrations_directory()
                        .map_err(handle_error).unwrap()
                       );

    setup_database(matches).unwrap_or_else(handle_error);
}

fn run_database_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("setup", Some(args)) => setup_database(args).unwrap_or_else(handle_error),
        ("reset", Some(args)) => reset_database(args).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
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

/// Creates the __diesel_schema_migrations table if it doesn't exist. If the
/// table didn't exist, it also runs any pending migrations. Returns a
/// `DatabaseError::ConnectionError` if it can't create the table, and exits
/// with a migration error if it can't run migrations.
fn create_schema_table_and_run_migrations_if_needed(database_url: &String)
    -> DatabaseResult<()>
{
    if !schema_table_exists(database_url).map_err(handle_error).unwrap() {
        try!(call_with_conn!(database_url, migrations::create_schema_migrations_table_if_needed));
        call_with_conn!(database_url, migrations::run_pending_migrations).unwrap_or_else(handle_error);
    };
    Ok(())
}

/// Drops the database specified in the connection url. It returns an error
/// if it was unable to drop the database.
fn drop_database(database_url: &String) -> DatabaseResult<()> {
    match backend(database_url) {
        "postgres" => {
            let (database, postgres_url) = split_pg_connection_string(database_url);
            println!("Dropping database: {}", database);
            let conn = try!(PgConnection::establish(&postgres_url));
            try!(conn.silence_notices(|| {
                conn.execute(&format!("DROP DATABASE IF EXISTS {}", database))
            }));
        },
        "sqlite" => {
            println!("Dropping database: {}", database_url);
            try!(fs::remove_file(&database_url));
        },
        _ => unreachable!("The backend function should ensure we never get here."),
    }
    Ok(())
}

/// Creates the database specified in the connection url. It returns an error
/// it it was unable to create the database.
fn create_database_if_needed(database_url: &String)
    -> DatabaseResult<()>
{
    match backend(database_url) {
        "postgres" => {
            if PgConnection::establish(database_url).is_err() {
                let(database, postgres_url) = split_pg_connection_string(database_url);
                try!(create_postgres_database(&postgres_url, &database));
            }
        },
        "sqlite" => {
            println!("Creating database: {}", database_url);
            try!(SqliteConnection::establish(database_url));
        },
        _ => unreachable!("The backend function should ensure we never get here."),
    }

    Ok(())
}

fn create_postgres_database(database_url: &String, database: &String)
    -> DatabaseResult<()>
{
    let conn = try!(PgConnection::establish(database_url));
    println!("Creating database: {}", database);
    try!(conn.execute(&format!("CREATE DATABASE {}", database)));
    Ok(())
}

/// Looks for a migrations directory in the current path and all parent paths,
/// and creates one in the same directory as the Cargo.toml if it can't find
/// one. It also sticks a .gitkeep in the directory so git will pick it up.
/// Returns a `DatabaseError::CargoTomlNotFound` if no Cargo.toml is found.
fn create_migrations_directory() -> DatabaseResult<PathBuf> {
    let project_root = try!(find_project_root());
    println!("Creating migrations/ directory at: {}", project_root
                                                        .join("migrations")
                                                        .display());
    try!(fs::create_dir(project_root.join("migrations")));
    try!(fs::File::create(project_root.join("migrations/.gitkeep")));
    Ok(project_root)
}

fn find_project_root() -> DatabaseResult<PathBuf> {
    search_for_cargo_toml_directory(&try!(env::current_dir()))
}

fn split_pg_connection_string(database_url: &String) -> (String, String) {
    let mut split: Vec<&str> = database_url.split("/").collect();
    let database = split.pop().unwrap();
    let postgres_url = split.join("/");
    (database.to_owned(), postgres_url)
}

/// Searches for the directory that holds the project's Cargo.toml, and returns
/// the path if it found it, or returns a `DatabaseError::CargoTomlNotFound`.
fn search_for_cargo_toml_directory(path: &Path) -> DatabaseResult<PathBuf> {
    let toml_path = path.join("Cargo.toml");
    if toml_path.is_file() {
        Ok(path.to_owned())
    } else {
        path.parent().map(search_for_cargo_toml_directory)
            .unwrap_or(Err(DatabaseError::CargoTomlNotFound))
    }
}

/// Returns true if the '__diesel_schema_migrations' table exists in the
/// database we connect to, returns false if it does not.
pub fn schema_table_exists(database_url: &String) -> DatabaseResult<bool> {
    let result = match backend(database_url) {
        "postgres" => {
            let conn = PgConnection::establish(database_url).unwrap();
            try!(select(sql::<Bool>("EXISTS \
                    (SELECT 1 \
                     FROM information_schema.tables \
                     WHERE table_name = '__diesel_schema_migrations')"))
                .get_result(&conn))
        },
        "sqlite" => {
            let conn = SqliteConnection::establish(database_url).unwrap();
            try!(select(sql::<Bool>("EXISTS \
                    (SELECT 1 \
                     FROM sqlite_master \
                     WHERE type = 'table' \
                     AND name = '__diesel_schema_migrations')"))
                .get_result(&conn))
        },
        _ => unreachable!("The backend function should ensure we never get here."),
    };
    Ok(result)
}

/// Reverts the most recent migration, and then runs it again, all in a
/// transaction. If either part fails, the transaction is not committed.
fn redo_latest_migration<Conn>(conn: &Conn) where
        Conn: Connection,
        String: FromSql<VarChar, Conn::Backend>,
        for<'a> &'a NewMigration<'a>:
            Insertable<__diesel_schema_migrations::table, Conn::Backend>,
{
    conn.transaction(|| {
        let reverted_version = try!(migrations::revert_latest_migration(conn));
        migrations::run_migration_with_version(conn, &reverted_version, &mut stdout())
    }).unwrap_or_else(handle_error);
}

fn handle_error<E: Error>(error: E) {
    panic!("{}", error);
}

pub fn database_url(matches: &ArgMatches) -> String {
    dotenv::dotenv().ok();

    matches.value_of("DATABASE_URL")
        .map(|s| s.into())
        .or(env::var("DATABASE_URL").ok())
        .expect("The --database-url argument must be passed, \
                or the DATABASE_URL environment variable must be set.")
}

/// Returns a &str representing the type of backend being used, determined
/// by the format of the database url.
pub fn backend(database_url: &String) -> &str {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        "postgres"
    } else {
        "sqlite"
    }
}

// Converts an absolute path to a relative path, with the restriction that the
// target path must be in the same directory or above the current path.
fn convert_absolute_path_to_relative(target_path: &mut PathBuf, current_path: &mut PathBuf)
    -> PathBuf
{
    let mut result = PathBuf::new();
    let target_path = target_path.as_path();
    let mut current_path = current_path.as_path();

    while !target_path.starts_with(current_path) {
        result.push("..");
        current_path = current_path.parent().unwrap();
    }

    result.join(strip_prefix(target_path, current_path))
}

// FIXME: Remove all of this when 1.7 is stable
fn strip_prefix<'a>(target: &'a Path, base: &'a Path)
-> &'a Path {
    iter_after(target.components(), base.components())
        .map(|c| c.as_path()).unwrap()
}

fn iter_after<A, I, J>(mut iter: I, mut prefix: J) -> Option<I>
where I: Iterator<Item = A> + Clone,
      J: Iterator<Item = A>,
      A: PartialEq
{
    loop {
        let mut iter_next = iter.clone();
        match (iter_next.next(), prefix.next()) {
            (Some(x), Some(y)) => {
                if x != y {
                    return None;
                }
            }
            (Some(_), None) => return Some(iter),
            (None, None) => return Some(iter),
            (None, Some(_)) => return None,
        }
        iter = iter_next;
    }
}
// End FIXME

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use dotenv::dotenv;
    use diesel::Connection;

    use database_error::DatabaseError;

    use self::tempdir::TempDir;

    use std::{env, fs};
    use std::path::PathBuf;

    use super::convert_absolute_path_to_relative;
    use super::search_for_cargo_toml_directory;
    use super::{create_database_if_needed, drop_database, schema_table_exists};
    use super::split_pg_connection_string;

    #[cfg(feature = "postgres")]
    type TestConnection = ::diesel::pg::PgConnection;
    #[cfg(feature = "sqlite")]
    type TestConnection = ::diesel::sqlite::SqliteConnection;

    type TestBackend = <TestConnection as Connection>::Backend;

    #[cfg(feature = "postgres")]
    fn database_url(identifier: &str) -> String {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in order to run diesel_cli tests");
        let (_, base_url) = split_pg_connection_string(&database_url);
        format!("{}/{}", base_url, identifier)
    }

    #[cfg(feature = "postgres")]
    fn connection(database_url: &String) -> TestConnection {
        ::diesel::pg::PgConnection::establish(database_url).unwrap()
    }

    #[cfg(feature = "postgres")]
    fn teardown(_: String) {
    }

    #[cfg(feature = "sqlite")]
    fn database_url(identifier: &str) -> String {
        let dir = env::current_dir().unwrap();
        let db_file = dir.join(identifier);
        fs::remove_file(&db_file).ok();
        fs::File::create(&db_file).unwrap();
        db_file.to_str().unwrap().to_owned()
    }

    #[cfg(feature = "sqlite")]
    fn connection(database_url: &String) -> TestConnection {
        ::diesel::sqlite::SqliteConnection::establish(database_url).unwrap()
    }

    #[cfg(feature = "sqlite")]
    fn teardown(database_path: String) {
        fs::remove_file(&database_path).unwrap();
    }

    #[test]
    fn schema_table_exists_finds_table() {
        let database_url = database_url("test1");
        create_database_if_needed(&database_url)
            .expect("Unable to create test database");
        let connection = connection(&database_url);
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations").unwrap();
            connection.execute("CREATE TABLE __diesel_schema_migrations (version INTEGER)").unwrap();
        });

        assert!(schema_table_exists(&database_url).unwrap());

        teardown(database_url);
    }

    #[test]
    fn schema_table_exists_doesnt_find_table() {
        let database_url = database_url("test2");
        create_database_if_needed(&database_url)
            .expect("Unable to create test database");
        let connection = connection(&database_url);
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations").unwrap();
        });

        assert!(!schema_table_exists(&database_url).unwrap());

        teardown(database_url);
    }

    #[test]
    fn create_database_creates_the_database() {
        let database_url = database_url("test3");
        drop_database(&database_url).unwrap();
        create_database_if_needed(&database_url).unwrap();
        assert!(TestConnection::establish(&database_url).is_ok());

        teardown(database_url);
    }

    #[test]
    #[cfg(feature = "postgres")]
    fn drop_database_drops_the_database() {
        let database_url = database_url("test4");
        drop_database(&database_url).unwrap();
        assert!(TestConnection::establish(&database_url).is_err());
    }

    #[test]
    #[cfg(feature = "sqlite")]
    fn drop_database_drops_the_database() {
        let database_url = database_url("test4");
        drop_database(&database_url).unwrap();
        assert!(fs::File::open(database_url).is_err());
    }

    // #[test]
    // #[should_panic] // FIXME: Our migration structure is non-standard
    // // we need to test this against a clean env with a normal structure
    // // once we get integration test coverage (we can't change cwd in the test
    // // process)
    // fn create_schema_table_creates_diesel_table() {
    //     let database_url = database_url("test5");
    //     create_database_if_needed(&database_url)
    //         .expect("Unable to create test database");
    //     let connection = connection(&database_url);
    //     connection.silence_notices(|| {
    //         connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations").unwrap();
    //     });
    //     assert!(!schema_table_exists(&database_url).unwrap());
    //     create_schema_table_and_run_migrations_if_needed(&database_url).unwrap();
    //     assert!(schema_table_exists(&database_url).unwrap());
    //
    //     teardown(database_url);
    // }

    #[test]
    fn split_pg_connection_string_returns_postgres_url_and_database() {
        let database = "database".to_owned();
        let postgres_url = "postgresql://localhost:5432".to_owned();
        let database_url = format!("{}/{}", postgres_url, database);
        assert_eq!((database, postgres_url), split_pg_connection_string(&database_url));
    }

    #[test]
    fn toml_directory_find_cargo_toml() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let toml_path = temp_path.join("Cargo.toml");

        fs::File::create(&toml_path).unwrap();

        assert_eq!(Ok(temp_path.clone()), search_for_cargo_toml_directory(&temp_path));
    }

    #[test]
    fn cargo_toml_not_found_if_no_cargo_toml() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();

        assert_eq!(Err(DatabaseError::CargoTomlNotFound),
            search_for_cargo_toml_directory(&temp_path));
    }

    #[test]
    fn convert_absolute_path_to_relative_works() {
        assert_eq!(PathBuf::from("migrations/12345_create_user"),
                        convert_absolute_path_to_relative(&mut PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                            &mut PathBuf::from("projects/foo")));
        assert_eq!(PathBuf::from("../migrations/12345_create_user"),
                        convert_absolute_path_to_relative(&mut PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                            &mut PathBuf::from("projects/foo/src")));
        assert_eq!(PathBuf::from("../../../migrations/12345_create_user"),
                        convert_absolute_path_to_relative(&mut PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                            &mut PathBuf::from("projects/foo/src/controllers/errors")));
        assert_eq!(PathBuf::from("12345_create_user"),
                        convert_absolute_path_to_relative(&mut PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                            &mut PathBuf::from("projects/foo/migrations")));
        assert_eq!(PathBuf::from("../12345_create_user"),
                        convert_absolute_path_to_relative(&mut PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                            &mut PathBuf::from("projects/foo/migrations/67890_create_post")));
    }
}
