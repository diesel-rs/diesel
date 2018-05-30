// Built-in Lints
#![deny(warnings, missing_copy_implementations)]
// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(option_map_unwrap_or_else, option_map_unwrap_or))]
#![cfg_attr(
    feature = "clippy",
    warn(
        wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names, unicode_not_nfc,
        if_not_else, items_after_statements, used_underscore_binding
    )
)]
#![cfg_attr(all(test, feature = "clippy"), allow(result_unwrap_used))]

extern crate chrono;
extern crate clap;
#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate migrations_internals;
#[macro_use]
extern crate serde;
extern crate tempfile;
extern crate toml;
#[cfg(feature = "url")]
extern crate url;

mod config;

#[cfg(feature = "barrel-migrations")]
extern crate barrel;

mod database_error;
#[macro_use]
mod database;
mod cli;
mod infer_schema_internals;
mod print_schema;
#[cfg(any(feature = "postgres", feature = "mysql"))]
mod query_helper;

use chrono::*;
use clap::{ArgMatches, Shell};
use migrations_internals::{self as migrations, MigrationConnection};
use std::any::Any;
use std::error::Error;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::{env, fs};

use self::config::Config;
use self::database_error::{DatabaseError, DatabaseResult};
use migrations_internals::TIMESTAMP_FORMAT;

fn main() {
    use self::dotenv::dotenv;
    dotenv().ok();

    let matches = cli::build_cli().get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches).unwrap_or_else(handle_error),
        ("setup", Some(matches)) => run_setup_command(matches),
        ("database", Some(matches)) => run_database_command(matches),
        ("bash-completion", Some(matches)) => generate_bash_completion_command(matches),
        ("print-schema", Some(matches)) => run_infer_schema(matches).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn run_migration_command(matches: &ArgMatches) -> Result<(), Box<Error>> {
    match matches.subcommand() {
        ("run", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches);
            call_with_conn!(
                database_url,
                migrations::run_pending_migrations_in_directory(&dir, &mut stdout())
            )?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("revert", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches);
            call_with_conn!(
                database_url,
                migrations::revert_latest_migration_in_directory(&dir)
            )?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("redo", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches);
            call_with_conn!(database_url, redo_latest_migration(&dir));
            regenerate_schema_if_file_specified(matches)?;
        }
        ("list", Some(_)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(matches);
            let mut migrations =
                call_with_conn!(database_url, migrations::mark_migrations_in_directory(&dir))?;

            migrations.sort_by_key(|&(ref m, _)| m.version().to_string());

            println!("Migrations:");
            for (migration, applied) in migrations {
                let name = migration
                    .file_path()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy();
                let x = if applied { 'X' } else { ' ' };
                println!("  [{}] {}", x, name);
            }
        }
        ("pending", Some(_)) => {
            let database_url = database::database_url(matches);
            let result = call_with_conn!(database_url, migrations::any_pending_migrations)?;
            println!("{:?}", result);
        }
        ("generate", Some(args)) => {
            let migration_name = args.value_of("MIGRATION_NAME").unwrap();
            let version = migration_version(args);
            let versioned_name = format!("{}_{}", version, migration_name);
            let migration_dir = migrations_dir(matches).join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            match args.value_of("MIGRATION_FORMAT") {
                #[cfg(feature = "barrel-migrations")]
                Some("barrel") => ::barrel::integrations::diesel::generate_initial(&migration_dir),
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

use std::fmt::Display;
fn migration_version<'a>(matches: &'a ArgMatches) -> Box<Display + 'a> {
    matches
        .value_of("MIGRATION_VERSION")
        .map(|s| Box::new(s) as Box<Display>)
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

fn migrations_dir(matches: &ArgMatches) -> PathBuf {
    migrations_dir_from_cli(matches)
        .or_else(|| env::var("MIGRATION_DIRECTORY").map(PathBuf::from).ok())
        .unwrap_or_else(|| migrations::find_migrations_directory().unwrap_or_else(handle_error))
}

fn run_setup_command(matches: &ArgMatches) {
    let migrations_dir = create_migrations_dir(matches).unwrap_or_else(handle_error);
    create_config_file(matches).unwrap_or_else(handle_error);

    database::setup_database(matches, &migrations_dir).unwrap_or_else(handle_error);
}

fn create_migrations_dir(matches: &ArgMatches) -> DatabaseResult<PathBuf> {
    let dir = matches
        .value_of("MIGRATION_DIRECTORY")
        .map(PathBuf::from)
        .or_else(|| env::var("MIGRATION_DIRECTORY").map(PathBuf::from).ok())
        .unwrap_or_else(|| {
            find_project_root()
                .unwrap_or_else(handle_error)
                .join("migrations")
        });

    if !dir.exists() {
        create_migrations_directory(&dir)?;
    }

    Ok(dir.to_owned())
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

fn run_database_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("setup", Some(args)) => {
            let migrations_dir = migrations_dir(args);
            database::setup_database(args, &migrations_dir).unwrap_or_else(handle_error)
        }
        ("reset", Some(args)) => {
            let migrations_dir = migrations_dir(args);
            database::reset_database(args, &migrations_dir).unwrap_or_else(handle_error)
        }
        ("drop", Some(args)) => database::drop_database_command(args).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
    };
}

fn generate_bash_completion_command(_: &ArgMatches) {
    cli::build_cli().gen_completions_to("diesel", Shell::Bash, &mut stdout());
}

/// Looks for a migrations directory in the current path and all parent paths,
/// and creates one in the same directory as the Cargo.toml if it can't find
/// one. It also sticks a .gitkeep in the directory so git will pick it up.
/// Returns a `DatabaseError::CargoTomlNotFound` if no Cargo.toml is found.
fn create_migrations_directory(path: &Path) -> DatabaseResult<PathBuf> {
    println!("Creating migrations directory at: {}", path.display());
    fs::create_dir(path)?;
    fs::File::create(path.join(".gitkeep"))?;
    Ok(path.to_owned())
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
        path.parent()
            .map(search_for_cargo_toml_directory)
            .unwrap_or(Err(DatabaseError::CargoTomlNotFound))
    }
}

/// Reverts the most recent migration, and then runs it again, all in a
/// transaction. If either part fails, the transaction is not committed.
fn redo_latest_migration<Conn>(conn: &Conn, migrations_dir: &Path)
where
    Conn: MigrationConnection + Any,
{
    let migration_inner = || {
        let reverted_version = try!(migrations::revert_latest_migration_in_directory(
            conn,
            migrations_dir
        ));
        migrations::run_migration_with_version(
            conn,
            migrations_dir,
            &reverted_version,
            &mut stdout(),
        )
    };
    if should_redo_migration_in_transaction(conn) {
        conn.transaction(migration_inner)
            .unwrap_or_else(handle_error);
    } else {
        migration_inner().unwrap_or_else(handle_error);
    }
}

#[cfg(feature = "mysql")]
fn should_redo_migration_in_transaction(t: &Any) -> bool {
    !t.is::<::diesel::mysql::MysqlConnection>()
}

#[cfg(not(feature = "mysql"))]
fn should_redo_migration_in_transaction(_t: &Any) -> bool {
    true
}

#[cfg_attr(feature = "clippy", allow(needless_pass_by_value))]
fn handle_error<E: Display, T>(error: E) -> T {
    println!("{}", error);
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

fn run_infer_schema(matches: &ArgMatches) -> Result<(), Box<Error>> {
    use infer_schema_internals::TableName;
    use print_schema::*;

    let database_url = database::database_url(matches);
    let mut config = Config::read(matches)?.print_schema;

    if let Some(schema_name) = matches.value_of("schema") {
        config.schema = Some(String::from(schema_name))
    }

    let filter = matches
        .values_of("table-name")
        .unwrap_or_default()
        .map(|table_name| {
            if let Some(schema) = config.schema_name() {
                TableName::new(table_name, schema)
            } else {
                table_name.parse().unwrap()
            }
        })
        .collect();

    if matches.is_present("whitelist") {
        eprintln!("The `whitelist` option has been deprecated and renamed to `only-tables`.");
    }

    if matches.is_present("blacklist") {
        eprintln!("The `blacklist` option has been deprecated and renamed to `except-tables`.");
    }

    if matches.is_present("only-tables") || matches.is_present("whitelist") {
        config.filter = Filtering::OnlyTables(filter)
    } else if matches.is_present("except-tables") || matches.is_present("blacklist") {
        config.filter = Filtering::ExceptTables(filter)
    }

    if matches.is_present("with-docs") {
        config.with_docs = true;
    }

    if let Some(path) = matches.value_of("patch-file") {
        config.patch_file = Some(PathBuf::from(path));
    }

    if let Some(types) = matches.values_of("import-types") {
        let types = types.map(String::from).collect();
        config.import_types = Some(types);
    }

    run_print_schema(&database_url, &config)?;
    Ok(())
}

fn regenerate_schema_if_file_specified(matches: &ArgMatches) -> Result<(), Box<Error>> {
    let config = Config::read(matches)?;
    if let Some(ref path) = config.print_schema.file {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let database_url = database::database_url(matches);
        let mut file = fs::File::create(path)?;
        print_schema::output_schema(&database_url, &config.print_schema, file, path)?;
    }
    Ok(())
}

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

        assert_eq!(
            Ok(temp_path.clone()),
            search_for_cargo_toml_directory(&temp_path)
        );
    }

    #[test]
    fn cargo_toml_not_found_if_no_cargo_toml() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();

        assert_eq!(
            Err(DatabaseError::CargoTomlNotFound),
            search_for_cargo_toml_directory(&temp_path)
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
