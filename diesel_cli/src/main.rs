extern crate chrono;
extern crate clap;
extern crate diesel;
extern crate dotenv;

mod database_error;
#[macro_use]
mod database;

use chrono::*;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use diesel::pg::PgConnection;
use diesel::sqlite::SqliteConnection;
use diesel::migrations::schema::*;
use diesel::types::{FromSql, VarChar};
use diesel::{migrations, Connection, Insertable};
use std::error::Error;
use std::io::stdout;
use std::path::{PathBuf, Path};
use std::{env, fs};

use self::database_error::{DatabaseError, DatabaseResult};

fn main() {
    let database_arg = Arg::with_name("DATABASE_URL")
        .long("database-url")
        .help("Specifies the database URL to connect to. Falls back to \
                   the DATABASE_URL environment variable if unspecified.")
        .global(true)
        .takes_value(true);

    let migration_subcommand = SubCommand::with_name("migration")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs all pending migrations")
        ).subcommand(
            SubCommand::with_name("revert")
                .about("Reverts the latest run migration")
        ).subcommand(
            SubCommand::with_name("redo")
                .about("Reverts and re-runs the latest migration. Useful \
                      for testing that a migration can in fact be reverted.")
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
                specified in your DATABASE_URL, and runs existing migrations.");

    let database_subcommand = SubCommand::with_name("database")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("setup")
                .about("Creates the migrations directory, creates the database \
                        specified in your DATABASE_URL, and runs existing migrations.")
        ).subcommand(
            SubCommand::with_name("reset")
                .about("Resets your database by dropping the database specified \
                        in your DATABASE_URL and then running `diesel database setup`.")
        ).subcommand(
            SubCommand::with_name("drop")
                .about("Drops the database specified in your DATABASE_URL")
                .setting(AppSettings::Hidden)
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .arg(database_arg)
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
    let database_url = database::database_url(matches);

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

    database::setup_database(matches).unwrap_or_else(handle_error);
}

fn run_database_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("setup", Some(args)) => database::setup_database(args).unwrap_or_else(handle_error),
        ("reset", Some(args)) => database::reset_database(args).unwrap_or_else(handle_error),
        ("drop", Some(args)) => database::drop_database_command(args).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
    };
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

    use database_error::DatabaseError;

    use self::tempdir::TempDir;

    use std::fs;
    use std::path::PathBuf;

    use super::convert_absolute_path_to_relative;
    use super::search_for_cargo_toml_directory;

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
