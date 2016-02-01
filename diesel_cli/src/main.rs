extern crate chrono;
#[macro_use]
extern crate clap;
extern crate diesel;

mod database_error;

use chrono::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use diesel::{migrations, Connection};
use diesel::connection::PgConnection;
use self::database_error::DatabaseError;
use std::{env, fs};
use std::error::Error;
use std::path::{PathBuf, Path};

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
        _ => unreachable!(),
    }
}

fn run_migration_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("run", Some(args)) => {
            migrations::run_pending_migrations(&connection(&database_url(args)))
                .map_err(handle_error).unwrap();
        }
        ("revert", Some(args)) => {
            migrations::revert_latest_migration(&connection(&database_url(args)))
                .map_err(handle_error).unwrap();
        }
        ("redo", Some(args)) => {
            let connection = connection(&database_url(args));
            connection.transaction(|| {
                let reverted_version = try!(migrations::revert_latest_migration(&connection));
                migrations::run_migration_with_version(&connection, &reverted_version)
            }).unwrap_or_else(handle_error);
        }
        ("generate", Some(args)) => {
            let migration_name = args.value_of("MIGRATION_NAME").unwrap();
            let timestamp = Local::now().format("%Y%m%d%H%M%S");
            let versioned_name = format!("{}_{}", &timestamp, migration_name);
            let migration_dir = migrations::find_migrations_directory()
                .map_err(handle_error).unwrap().join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            // FIXME: It would be nice to print these as relative paths
            let up_path = migration_dir.join("up.sql");
            println!("Creating {}", up_path.display());
            fs::File::create(up_path).unwrap();
            let down_path = migration_dir.join("down.sql");
            println!("Creating {}", down_path.display());
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

fn reset_database(args: &ArgMatches) -> Result<(), DatabaseError> {
    let (database, postgres_url) = split_pg_connection_string(&database_url(args));
    try!(drop_database(&postgres_url, &database));
    setup_database(args)
}

fn setup_database(args: &ArgMatches) -> Result<(), DatabaseError> {
    let database_url = database_url(args);

    if PgConnection::establish(&database_url).is_err() {
        let (database, postgres_url) = split_pg_connection_string(&database_url);
        try!(create_database(&postgres_url, &database));
    }

    let connection = connection(&database_url);
    create_schema_table_and_run_migrations_if_needed(&connection)
}

/// Creates the __diesel_schema_migrations table if it doesn't exist. If the
/// table didn't exist, it also runs any pending migrations. Returns a
/// `DatabaseError::ConnectionError` if it can't create the table, and exits
/// with a migration error if it can't run migrations.
fn create_schema_table_and_run_migrations_if_needed<Conn: Connection>(conn: &Conn)
    -> Result<(), DatabaseError>
{
    if !schema_table_exists(conn).map_err(handle_error).unwrap() {
        try!(migrations::create_schema_migrations_table_if_needed(conn));
        migrations::run_pending_migrations(conn).unwrap_or_else(handle_error);
    };
    Ok(())
}

/// Drops the database specified in the connection url. It returns an error
/// if it was unable to drop the database.
fn drop_database(database_url: &String, database: &String)
    -> Result<(), DatabaseError>
{
    let conn = try!(PgConnection::establish(database_url));
    println!("Dropping database: {}", database);
    try!(conn.silence_notices(|| {
           conn.execute(&format!("DROP DATABASE IF EXISTS {};", database))
    }));
    Ok(())
}

/// Creates the database specified in the connection url. It returns an error
/// it it was unable to create the database.
fn create_database(database_url: &String, database: &String)
    -> Result<(), DatabaseError>
{
    let conn = try!(PgConnection::establish(database_url));
    println!("Creating database: {}", database);
    try!(conn.execute(&format!("CREATE DATABASE {};", database)));
    Ok(())
}

/// Looks for a migrations directory in the current path and all parent paths,
/// and creates one in the same directory as the Cargo.toml if it can't find
/// one. It also sticks a .gitkeep in the directory so git will pick it up.
/// Returns a `DatabaseError::CargoTomlNotFound` if no Cargo.toml is found.
fn create_migrations_directory() -> Result<PathBuf, DatabaseError> {
    let project_root = try!(find_project_root());
    try!(fs::create_dir(project_root.join("migrations")));
    try!(fs::File::create(project_root.join("migrations/.gitkeep")));
    println!("Created migrations/ directory at: {}", project_root
                                                        .join("migrations")
                                                        .display());
    Ok(project_root)
}

fn find_project_root() -> Result<PathBuf, DatabaseError> {
    search_for_cargo_toml_directory(&try!(env::current_dir()))
}

// TODO: This will need to be made generic along with the rest of the migrations
// and the CLI, which is why this specifically has pg in the name.
fn split_pg_connection_string(database_url: &String) -> (String, String) {
    let mut split: Vec<&str> = database_url.split("/").collect();
    let database = split.pop().unwrap();
    let postgres_url = split.join("/");
    (database.to_owned(), postgres_url)
}

/// Searches for the directory that holds the project's Cargo.toml, and returns
/// the path if it found it, or returns a `DatabaseError::CargoTomlNotFound`.
fn search_for_cargo_toml_directory(path: &Path) -> Result<PathBuf, DatabaseError> {
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
pub fn schema_table_exists<Conn: Connection>(conn: &Conn) -> Result<bool, DatabaseError> {
    let result = try!(conn.execute("SELECT 1
        FROM information_schema.tables
        WHERE table_name = '__diesel_schema_migrations';"));
    Ok(result != 0)
}

fn handle_error<E: Error>(error: E) {
    println!("{}", error);
    std::process::exit(1);
}

fn database_url(matches: &ArgMatches) -> String {
    matches.value_of("DATABASE_URL")
        .map(|s| s.into())
        .or(env::var("DATABASE_URL").ok())
        .expect("The --database-url argument must be passed, \
                or the DATABASE_URL environment variable must be set.")
}

fn connection(database_url: &str) -> PgConnection {
    PgConnection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

#[cfg(test)]
mod tests {
    extern crate diesel;
    extern crate dotenv;
    extern crate tempdir;

    use self::tempdir::TempDir;
    use self::diesel::Connection;
    use self::diesel::connection::PgConnection;

    use database_error::DatabaseError;

    use super::create_schema_table_and_run_migrations_if_needed;
    use super::{drop_database, create_database, split_pg_connection_string};
    use super::{schema_table_exists, search_for_cargo_toml_directory};

    use std::{env, fs};

    fn database_url() -> String {
        dotenv::dotenv().ok();
        env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in order to run diesel_cli tests")
    }

    fn connection_with_transaction(database_url: &String) -> PgConnection {
        let connection = PgConnection::establish(&database_url).unwrap();
        connection.begin_test_transaction().unwrap();
        connection
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
    fn schema_table_exists_finds_table() {
        let connection = connection_with_transaction(&database_url());
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations;").unwrap();
            connection.execute("CREATE TABLE __diesel_schema_migrations ();").unwrap();
        });

        assert!(schema_table_exists(&connection).unwrap());
    }

    #[test]
    fn schema_table_exists_doesnt_find_table() {
        let connection = connection_with_transaction(&database_url());
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations;").unwrap();
        });
        assert!(!schema_table_exists(&connection).unwrap());
    }

    #[test]
    fn create_database_creates_the_database() {
        let (_, postgres_url) = split_pg_connection_string(&database_url());
        let database = "__diesel_test_database".to_owned();
        drop_database(&postgres_url, &database).unwrap();
        create_database(&postgres_url, &database).unwrap();
        let database_url = format!("{}/{}", postgres_url, database);
        assert!(PgConnection::establish(&database_url).is_ok());
    }

    #[test]
    fn drop_database_drops_the_database() {
        let (_, postgres_url) = split_pg_connection_string(&database_url());
        let database = "__diesel_cli_test_database".to_owned();
        drop_database(&postgres_url, &database).unwrap();
        let database_url = format!("{}/{}", postgres_url, database);
        assert!(PgConnection::establish(&database_url).is_err());
    }

    #[test]
    fn drop_database_handles_database_not_existing() {
        let (_, postgres_url) = split_pg_connection_string(&database_url());
        let database = "__diesel_cli_test_database".to_owned();
        drop_database(&postgres_url, &database).unwrap();
        drop_database(&postgres_url, &database).unwrap();
        let database_url = format!("{}/{}", postgres_url, database);
        assert!(PgConnection::establish(&database_url).is_err());
    }

    #[test]
    fn create_schema_table_creates_diesel_table() {
        let connection = connection_with_transaction(&database_url());
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations;").unwrap();
        });
        create_schema_table_and_run_migrations_if_needed(&connection).unwrap();
        assert!(schema_table_exists(&connection).unwrap());
    }

    #[test]
    fn split_pg_connection_string_returns_postgres_url_and_database() {
        let database = "database".to_owned();
        let postgres_url = "postgresql://localhost:5432".to_owned();
        let database_url = format!("{}/{}", postgres_url, database);
        assert_eq!((database, postgres_url), split_pg_connection_string(&database_url));
    }
}
