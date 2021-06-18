// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![allow(clippy::map_unwrap_or)]
#![warn(
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::used_underscore_binding,
    clippy::wrong_pub_self_convention
)]
#![cfg_attr(test, allow(clippy::result_unwrap_used))]

mod config;

mod database_error;
#[macro_use]
mod database;
mod cli;
mod infer_schema_internals;
mod print_schema;
#[cfg(any(feature = "postgres", feature = "mysql"))]
mod query_helper;
mod validators;

use chrono::*;
use clap::{ArgMatches, Shell};
use diesel::backend::Backend;
use diesel::migration::MigrationSource;
use diesel::Connection;
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationError, MigrationHarness};
use regex::Regex;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Display;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::{env, fs};

use self::config::Config;
use self::database_error::{DatabaseError, DatabaseResult};
pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

fn main() {
    use dotenv::dotenv;
    dotenv().ok();

    let matches = cli::build_cli().get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches).unwrap_or_else(handle_error),
        ("setup", Some(matches)) => run_setup_command(matches),
        ("database", Some(matches)) => run_database_command(matches).unwrap_or_else(handle_error),
        ("bash-completion", Some(matches)) => generate_bash_completion_command(matches),
        ("completions", Some(matches)) => generate_completions_command(matches),
        ("print-schema", Some(matches)) => run_infer_schema(matches).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn run_migration_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match matches.subcommand() {
        ("run", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            call_with_conn!(database_url, run_migrations_with_output(dir))?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("revert", Some(args)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            if args.is_present("REVERT_ALL") {
                call_with_conn!(database_url, revert_all_migrations_with_output(dir))?;
            } else {
                // TODO : remove this logic when upgrading to clap 3.0.
                // We handle the default_value here instead of doing it
                // in the cli. This is because arguments with default
                // values conflict even if not used.
                // See https://github.com/clap-rs/clap/issues/1605
                let number = match args.value_of("REVERT_NUMBER") {
                    None => "1",
                    Some(number) => number,
                };
                for _ in 0..number.parse::<u64>().expect("Unable to parse the value of the --number argument. A positive integer is expected.") {
                        match call_with_conn!(
                            database_url,
                            revert_migration_with_output(dir.clone())
                        ) {
                            Ok(_) => {}
                            Err(e) if e.is::<MigrationError>() => {
                                match e.downcast_ref::<MigrationError>() {
                                    // If n is larger then the actual number of migrations,
                                    // just stop reverting them
                                    Some(MigrationError::NoMigrationRun) => break,
                                    _ => return Err(e),
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
            }

            regenerate_schema_if_file_specified(matches)?;
        }
        ("redo", Some(args)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            call_with_conn!(database_url, redo_migrations(dir, args));
            regenerate_schema_if_file_specified(matches)?;
        }
        ("list", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            call_with_conn!(database_url, list_migrations(dir))?;
        }
        ("pending", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            let result =
                call_with_conn!(database_url, MigrationHarness::has_pending_migration(dir))?;
            println!("{:?}", result);
        }
        ("generate", Some(args)) => {
            let migration_name = args.value_of("MIGRATION_NAME").unwrap();
            let version = migration_version(args);
            let versioned_name = format!("{}_{}", version, migration_name);
            let migration_dir = migrations_dir(matches)
                .unwrap_or_else(handle_error)
                .join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            match args.value_of("MIGRATION_FORMAT") {
                Some("sql") => generate_sql_migration(&migration_dir),
                Some(x) => return Err(format!("Unrecognized migration format `{}`", x).into()),
                None => unreachable!("MIGRATION_FORMAT has a default value"),
            }
        }
        _ => unreachable!("The cli parser should prevent reaching here"),
    };

    Ok(())
}

fn generate_sql_migration(path: &PathBuf) {
    use std::io::Write;

    let migration_dir_relative =
        convert_absolute_path_to_relative(path, &env::current_dir().unwrap());

    let up_path = path.join("up.sql");
    println!(
        "Creating {}",
        migration_dir_relative.join("up.sql").display()
    );
    let mut up = fs::File::create(up_path).unwrap();
    up.write_all(b"-- Your SQL goes here").unwrap();

    let down_path = path.join("down.sql");
    println!(
        "Creating {}",
        migration_dir_relative.join("down.sql").display()
    );
    let mut down = fs::File::create(down_path).unwrap();
    down.write_all(b"-- This file should undo anything in `up.sql`")
        .unwrap();
}

fn migration_version<'a>(matches: &'a ArgMatches) -> Box<dyn Display + 'a> {
    matches
        .value_of("MIGRATION_VERSION")
        .map(|s| Box::new(s) as Box<dyn Display>)
        .unwrap_or_else(|| Box::new(Utc::now().format(TIMESTAMP_FORMAT)))
}

fn migrations_dir_from_cli(matches: &ArgMatches) -> Option<PathBuf> {
    matches
        .value_of("MIGRATION_DIRECTORY")
        .map(PathBuf::from)
        .or_else(|| {
            matches
                .subcommand()
                .1
                .and_then(|s| migrations_dir_from_cli(s))
        })
}

fn run_migrations_with_output<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    HarnessWithOutput::write_to_stdout(conn)
        .run_pending_migrations(migrations)
        .map(|_| ())
}

fn revert_all_migrations_with_output<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    HarnessWithOutput::write_to_stdout(conn)
        .revert_all_migrations(migrations)
        .map(|_| ())
}

fn revert_migration_with_output<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    HarnessWithOutput::write_to_stdout(conn)
        .revert_last_migration(migrations)
        .map(|_| ())
}

fn list_migrations<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    let applied_migrations = conn
        .applied_migrations()?
        .into_iter()
        .collect::<HashSet<_>>();

    let mut migrations = MigrationSource::<DB>::migrations(&migrations)?;
    migrations.sort_unstable_by(|a, b| a.name().version().cmp(&b.name().version()));
    println!("Migrations:");
    for migration in migrations {
        let applied = applied_migrations.contains(&migration.name().version());
        let name = migration.name();
        let x = if applied { 'X' } else { ' ' };
        println!("  [{}] {}", x, name);
    }

    Ok(())
}

/// Checks for a migrations folder in the following order :
/// 1. From the CLI arguments
/// 2. From the MIGRATION_DIRECTORY environment variable
/// 3. From `diesel.toml` in the `migrations_directory` section
///
/// Else try to find the migrations directory with the
/// `find_migrations_directory` in the diesel_migrations crate.
///
/// Returns a `MigrationError::MigrationDirectoryNotFound` if
/// no path to the migration directory is found.
fn migrations_dir(matches: &ArgMatches) -> Result<PathBuf, MigrationError> {
    let migrations_dir = migrations_dir_from_cli(matches)
        .or_else(|| env::var("MIGRATION_DIRECTORY").map(PathBuf::from).ok())
        .or_else(|| {
            Some(
                Config::read(matches)
                    .unwrap_or_else(handle_error)
                    .migrations_directory?
                    .dir,
            )
        });

    match migrations_dir {
        Some(dir) => Ok(dir),
        None => FileBasedMigrations::find_migrations_directory().map(|p| p.path().to_path_buf()),
    }
}

fn run_setup_command(matches: &ArgMatches) {
    create_config_file(matches).unwrap_or_else(handle_error);
    let migrations_dir = create_migrations_dir(matches).unwrap_or_else(handle_error);

    database::setup_database(matches, &migrations_dir).unwrap_or_else(handle_error);
}

/// Checks if the migration directory exists, else creates it.
/// For more information see the `migrations_dir` function.
fn create_migrations_dir(matches: &ArgMatches) -> DatabaseResult<PathBuf> {
    let dir = match migrations_dir(matches) {
        Ok(dir) => dir,
        Err(_) => find_project_root()
            .unwrap_or_else(handle_error)
            .join("migrations"),
    };

    if dir.exists() {
        // This is a cleanup code for migrating from an
        // older version of diesel_cli that set a `.gitkeep` instead of a `.keep` file.
        // TODO: remove this after a few releases
        if let Ok(read_dir) = fs::read_dir(&dir) {
            if let Some(dir_entry) =
                read_dir
                    .filter_map(|entry| entry.ok())
                    .find(|entry| match entry.file_type() {
                        Ok(file_type) => file_type.is_file() && entry.file_name() == ".gitkeep",
                        Err(_) => false,
                    })
            {
                fs::remove_file(dir_entry.path()).unwrap_or_else(|err| {
                    eprintln!(
                        "WARNING: Unable to delete existing `migrations/.gitkeep`:\n{}",
                        err
                    )
                });
            }
        }
    } else {
        create_migrations_directory(&dir)?;
    }

    Ok(dir)
}

fn create_config_file(matches: &ArgMatches) -> DatabaseResult<()> {
    use std::io::Write;
    let path = Config::file_path(matches);
    if !path.exists() {
        let mut file = fs::File::create(path)?;
        file.write_all(include_bytes!("default_files/diesel.toml"))?;
    }

    Ok(())
}

fn run_database_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match matches.subcommand() {
        ("setup", Some(args)) => {
            let migrations_dir = migrations_dir(matches).unwrap_or_else(handle_error);
            database::setup_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("reset", Some(args)) => {
            let migrations_dir = migrations_dir(matches).unwrap_or_else(handle_error);
            database::reset_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("drop", Some(args)) => database::drop_database_command(args)?,
        _ => unreachable!("The cli parser should prevent reaching here"),
    };
    Ok(())
}

fn generate_bash_completion_command(_: &ArgMatches) {
    eprintln!(
        "WARNING: `diesel bash-completion` is deprecated, use `diesel completions bash` instead"
    );
    cli::build_cli().gen_completions_to("diesel", Shell::Bash, &mut stdout());
}

fn generate_completions_command(matches: &ArgMatches) {
    use clap::value_t;

    let shell = value_t!(matches, "SHELL", Shell).unwrap_or_else(|e| e.exit());
    cli::build_cli().gen_completions_to("diesel", shell, &mut stdout());
}

/// Looks for a migrations directory in the current path and all parent paths,
/// and creates one in the same directory as the Cargo.toml if it can't find
/// one. It also sticks a .keep in the directory so git will pick it up.
/// Returns a `DatabaseError::ProjectRootNotFound` if no Cargo.toml is found.
fn create_migrations_directory(path: &Path) -> DatabaseResult<PathBuf> {
    println!("Creating migrations directory at: {}", path.display());
    fs::create_dir(path)?;
    fs::File::create(path.join(".keep"))?;
    Ok(path.to_owned())
}

fn find_project_root() -> DatabaseResult<PathBuf> {
    let current_dir = env::current_dir()?;
    search_for_directory_containing_file(&current_dir, "diesel.toml")
        .or_else(|_| search_for_directory_containing_file(&current_dir, "Cargo.toml"))
}

/// Searches for the directory that holds the project's Cargo.toml, and returns
/// the path if it found it, or returns a `DatabaseError::ProjectRootNotFound`.
fn search_for_directory_containing_file(path: &Path, file: &str) -> DatabaseResult<PathBuf> {
    let toml_path = path.join(file);
    if toml_path.is_file() {
        Ok(path.to_owned())
    } else {
        path.parent()
            .map(|p| search_for_directory_containing_file(p, file))
            .unwrap_or_else(|| Err(DatabaseError::ProjectRootNotFound(path.into())))
            .map_err(|_| DatabaseError::ProjectRootNotFound(path.into()))
    }
}

/// Reverts all the migrations, and then runs them again, if the `--all`
/// argument is used. Otherwise it only redoes a specific number of migrations
/// if the `--number` argument is used.
/// Migrations are performed in a transaction. If either part fails,
/// the transaction is not committed.
fn redo_migrations<Conn, DB>(
    conn: &mut Conn,
    migrations_dir: FileBasedMigrations,
    args: &ArgMatches,
) where
    DB: Backend,
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
{
    let migrations_inner = |harness: &mut HarnessWithOutput<Conn, _>| -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let reverted_versions = if args.is_present("REDO_ALL") {
            harness.revert_all_migrations(migrations_dir.clone())?
        } else {
            // TODO : remove this logic when upgrading to clap 3.0.
            // We handle the default_value here instead of doing it
            // in the cli. This is because arguments with default
            // values conflict even if not used.
            // See https://github.com/clap-rs/clap/issues/1605
            let number = match args.value_of("REDO_NUMBER") {
                None => "1",
                Some(number) => number,
            };

            (0..number.parse::<u64>().expect("Unable to parse the value of the --number argument. A positive integer is expected."))
                .filter_map(|_|{
                    match harness.revert_last_migration(migrations_dir.clone()) {
                        Ok(v) => {
                            Some(Ok(v))
                        }
                        Err(e) if e.is::<MigrationError>() => {
                            match e.downcast_ref::<MigrationError>() {
                                // If n is larger then the actual number of migrations,
                                // just stop reverting them
                                Some(MigrationError::NoMigrationRun) => None,
                                _ => Some(Err(e)),
                            }
                        }
                        Err(e) => {
                            Some(Err(e))
                        }
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut migrations = MigrationSource::<DB>::migrations(&migrations_dir)?
            .into_iter()
            .map(|m| (m.name().version().as_owned(), m))
            .collect::<HashMap<_, _>>();

        let migrations = reverted_versions
            .into_iter()
            .map(|v| {
                migrations
                    .remove(&v)
                    .ok_or_else(|| MigrationError::UnknownMigrationVersion(v.as_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        harness.run_migrations(&migrations)?;

        Ok(())
    };

    if should_redo_migration_in_transaction(conn) {
        conn.transaction(|conn| migrations_inner(&mut HarnessWithOutput::write_to_stdout(conn)))
            .unwrap_or_else(handle_error);
    } else {
        migrations_inner(&mut HarnessWithOutput::write_to_stdout(conn))
            .unwrap_or_else(handle_error);
    }
}

#[cfg(feature = "mysql")]
fn should_redo_migration_in_transaction(t: &dyn Any) -> bool {
    !t.is::<::diesel::mysql::MysqlConnection>()
}

#[cfg(not(feature = "mysql"))]
fn should_redo_migration_in_transaction(_t: &dyn Any) -> bool {
    true
}

#[allow(clippy::needless_pass_by_value)]
fn handle_error<E: Display, T>(error: E) -> T {
    eprintln!("{}", error);
    ::std::process::exit(1);
}

// Converts an absolute path to a relative path, with the restriction that the
// target path must be in the same directory or above the current path.
fn convert_absolute_path_to_relative(target_path: &Path, mut current_path: &Path) -> PathBuf {
    let mut result = PathBuf::new();

    while !target_path.starts_with(current_path) {
        result.push("..");
        match current_path.parent() {
            Some(parent) => current_path = parent,
            None => return target_path.into(),
        }
    }

    result.join(target_path.strip_prefix(current_path).unwrap())
}

fn run_infer_schema(matches: &ArgMatches) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    use crate::print_schema::*;

    let database_url = database::database_url(matches);
    let mut config = Config::read(matches)?.print_schema;

    if let Some(schema_name) = matches.value_of("schema") {
        config.schema = Some(String::from(schema_name))
    }

    let filter = matches
        .values_of("table-name")
        .unwrap_or_default()
        .map(|table_name_regex| Regex::new(table_name_regex).map(Into::into))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("invalid argument for table filtering regex: {}", e));

    if matches.is_present("only-tables") {
        config.filter = Filtering::OnlyTables(filter?)
    } else if matches.is_present("except-tables") {
        config.filter = Filtering::ExceptTables(filter?)
    }

    if matches.is_present("with-docs") {
        config.with_docs = true;
    }

    if let Some(sorting) = matches.value_of("column-sorting") {
        match sorting {
            "ordinal_position" => config.column_sorting = ColumnSorting::OrdinalPosition,
            "name" => config.column_sorting = ColumnSorting::Name,
            _ => return Err(format!("Invalid column sorting mode: {}", sorting).into()),
        }
    }

    if let Some(path) = matches.value_of("patch-file") {
        config.patch_file = Some(PathBuf::from(path));
    }

    if let Some(types) = matches.values_of("import-types") {
        let types = types.map(String::from).collect();
        config.import_types = Some(types);
    }

    if matches.is_present("generate-custom-type-definitions") {
        config.generate_missing_sql_type_definitions = Some(false);
    }

    run_print_schema(&database_url, &config, &mut stdout())?;
    Ok(())
}

fn regenerate_schema_if_file_specified(
    matches: &ArgMatches,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    use std::io::Read;

    let config = Config::read(matches)?;
    if let Some(ref path) = config.print_schema.file {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let database_url = database::database_url(matches);

        if matches.is_present("LOCKED_SCHEMA") {
            let mut buf = Vec::new();
            print_schema::run_print_schema(&database_url, &config.print_schema, &mut buf)?;

            let mut old_buf = Vec::new();
            let mut file = fs::File::open(path)?;
            file.read_to_end(&mut old_buf)?;

            if buf != old_buf {
                return Err(format!(
                    "Command would result in changes to {}. \
                     Rerun the command locally, and commit the changes.",
                    path.display()
                )
                .into());
            }
        } else {
            use std::io::Write;

            let mut file = fs::File::create(path)?;
            let schema = print_schema::output_schema(&database_url, &config.print_schema)?;
            file.write_all(schema.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use crate::database_error::DatabaseError;

    use self::tempfile::Builder;

    use std::fs;
    use std::path::PathBuf;

    use super::convert_absolute_path_to_relative;
    use super::search_for_directory_containing_file;

    #[test]
    fn toml_directory_find_cargo_toml() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let toml_path = temp_path.join("Cargo.toml");

        fs::File::create(&toml_path).unwrap();

        assert_eq!(
            Ok(temp_path.clone()),
            search_for_directory_containing_file(&temp_path, "Cargo.toml")
        );
    }

    #[test]
    fn cargo_toml_not_found_if_no_cargo_toml() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
        let temp_path = dir.path().canonicalize().unwrap();

        assert_eq!(
            Err(DatabaseError::ProjectRootNotFound(temp_path.clone())),
            search_for_directory_containing_file(&temp_path, "Cargo.toml")
        );
    }

    #[test]
    fn convert_absolute_path_to_relative_works() {
        assert_eq!(
            PathBuf::from("migrations/12345_create_user"),
            convert_absolute_path_to_relative(
                &PathBuf::from("projects/foo/migrations/12345_create_user"),
                &PathBuf::from("projects/foo")
            )
        );
        assert_eq!(
            PathBuf::from("../migrations/12345_create_user"),
            convert_absolute_path_to_relative(
                &PathBuf::from("projects/foo/migrations/12345_create_user"),
                &PathBuf::from("projects/foo/src")
            )
        );
        assert_eq!(
            PathBuf::from("../../../migrations/12345_create_user"),
            convert_absolute_path_to_relative(
                &PathBuf::from("projects/foo/migrations/12345_create_user"),
                &PathBuf::from("projects/foo/src/controllers/errors")
            )
        );
        assert_eq!(
            PathBuf::from("12345_create_user"),
            convert_absolute_path_to_relative(
                &PathBuf::from("projects/foo/migrations/12345_create_user"),
                &PathBuf::from("projects/foo/migrations")
            )
        );
        assert_eq!(
            PathBuf::from("../12345_create_user"),
            convert_absolute_path_to_relative(
                &PathBuf::from("projects/foo/migrations/12345_create_user"),
                &PathBuf::from("projects/foo/migrations/67890_create_post")
            )
        );
    }
}
