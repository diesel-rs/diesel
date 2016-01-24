extern crate chrono;
#[macro_use]
extern crate clap;
extern crate diesel;

mod setup_error;

use chrono::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use diesel::{migrations, Connection};
use diesel::connection::PgConnection;
use self::setup_error::SetupError;
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

    let matches = App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(migration_subcommand)
        .subcommand(setup_subcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches),
        ("setup", Some(matches)) => run_setup_command(matches),
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

    if PgConnection::establish(&database_url(matches)).is_err() {
        create_database(database_url(matches)).unwrap_or_else(handle_error);
    }

    let connection = connection(&database_url(matches));

    if !schema_table_exists(&connection).map_err(handle_error).unwrap() {
        migrations::create_schema_migrations_table_if_needed(&connection)
            .map_err(handle_error).unwrap();
        migrations::run_pending_migrations(&connection)
            .map_err(handle_error).unwrap();
    }
}

/// Creates the database specified in the connection url. It returns a
/// `SetupError::ConnectionError` if it can't connect to the postgres
/// connection url, and returns a `SetupError::QueryError` if it is unable
/// to create the database.
fn create_database(database_url: String) -> Result<(), SetupError> {
    let mut split: Vec<&str> = database_url.split("/").collect();
    let database = split.pop().unwrap();
    let postgres_url = split.join("/");
    let connection = try!(PgConnection::establish(&postgres_url));
    try!(connection.execute(&format!("CREATE DATABASE {};", database)));
    Ok(())
}

/// Looks for a migrations directory in the current path and all parent paths,
/// and creates one in the same directory as the Cargo.toml if it can't find
/// one. It also sticks a .gitkeep in the directory so git will pick it up.
/// Returns a `SetupError::CargoTomlNotFound` if no Cargo.toml is found.
fn create_migrations_directory() -> Result<PathBuf, SetupError> {
    let project_root = try!(find_project_root());
    try!(fs::create_dir(project_root.join("migrations")));
    try!(fs::File::create(project_root.join("migrations/.gitkeep")));
    Ok(project_root)
}

fn find_project_root() -> Result<PathBuf, SetupError> {
    search_for_cargo_toml_directory(&try!(env::current_dir()))
}

fn search_for_cargo_toml_directory(path: &Path) -> Result<PathBuf, SetupError> {
    let toml_path = path.join("Cargo.toml");
    if toml_path.is_file() {
        Ok(path.to_owned())
    } else {
        path.parent().map(search_for_cargo_toml_directory)
            .unwrap_or(Err(SetupError::CargoTomlNotFound))
    }
}

/// Returns true if the '__diesel_schema_migrations' table exists in the
/// database we connect to, returns false if it does not.
pub fn schema_table_exists(connection: &PgConnection) -> Result<bool, SetupError> {
    let result = try!(connection.execute("SELECT 1
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

    use setup_error::SetupError;

    use super::{schema_table_exists, search_for_cargo_toml_directory};

    use std::{env, fs};

    fn connection() -> PgConnection {
        dotenv::dotenv().ok();
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in order to run diesel_cli tests");
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

        assert_eq!(Err(SetupError::CargoTomlNotFound),
            search_for_cargo_toml_directory(&temp_path));
    }

    #[test]
    fn schema_table_exists_finds_table() {
        let connection = connection();
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations;").unwrap();
            connection.execute("CREATE TABLE __diesel_schema_migrations ();").unwrap();
        });

        assert!(schema_table_exists(&connection).unwrap());
    }

    #[test]
    fn schema_table_exists_doesnt_find_table() {
        let connection = connection();
        connection.silence_notices(|| {
            connection.execute("DROP TABLE IF EXISTS __diesel_schema_migrations;").unwrap();
        });
        assert!(!schema_table_exists(&connection).unwrap());
    }
}
