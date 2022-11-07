// Built-in Lints
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
    missing_copy_implementations
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod config;

mod database_error;
#[macro_use]
mod database;
mod cli;
mod infer_schema_internals;
mod migrations;
mod print_schema;
#[cfg(any(feature = "postgres", feature = "mysql"))]
mod query_helper;

use clap::ArgMatches;
use clap_complete::{generate, Shell};
use database::InferConnection;
use diesel::backend::Backend;
use diesel::Connection;
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationHarness};
use regex::Regex;
use std::error::Error;
use std::fmt::Display;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::{env, fs};

use self::config::Config;
use self::database_error::{DatabaseError, DatabaseResult};
pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

fn main() {
    use dotenvy::dotenv;
    dotenv().ok();

    let matches = cli::build_cli().get_matches();

    match matches.subcommand().unwrap() {
        ("migration", matches) => {
            self::migrations::run_migration_command(matches).unwrap_or_else(handle_error)
        }
        ("setup", matches) => run_setup_command(matches),
        ("database", matches) => run_database_command(matches).unwrap_or_else(handle_error),
        ("completions", matches) => generate_completions_command(matches),
        ("print-schema", matches) => run_infer_schema(matches).unwrap_or_else(handle_error),
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
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

fn run_setup_command(matches: &ArgMatches) {
    create_config_file(matches).unwrap_or_else(handle_error);
    let migrations_dir = create_migrations_dir(matches).unwrap_or_else(handle_error);

    database::setup_database(matches, &migrations_dir).unwrap_or_else(handle_error);
}

/// Checks if the migration directory exists, else creates it.
/// For more information see the `migrations_dir` function.
fn create_migrations_dir(matches: &ArgMatches) -> DatabaseResult<PathBuf> {
    let dir = match self::migrations::migrations_dir(matches) {
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
                    eprintln!("WARNING: Unable to delete existing `migrations/.gitkeep`:\n{err}")
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
    match matches.subcommand().unwrap() {
        ("setup", args) => {
            let migrations_dir =
                self::migrations::migrations_dir(matches).unwrap_or_else(handle_error);
            database::setup_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("reset", args) => {
            let migrations_dir =
                self::migrations::migrations_dir(matches).unwrap_or_else(handle_error);
            database::reset_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("drop", args) => database::drop_database_command(args)?,
        _ => unreachable!("The cli parser should prevent reaching here"),
    };
    Ok(())
}

fn generate_completions_command(matches: &ArgMatches) {
    let shell: &Shell = matches.get_one("SHELL").expect("Shell is set here?");
    let mut app = cli::build_cli();
    let name = app.get_name().to_string();
    generate(*shell, &mut app, name, &mut stdout());
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

#[allow(clippy::needless_pass_by_value)]
fn handle_error<E: Display, T>(error: E) -> T {
    eprintln!("{error}");
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

    let mut conn = InferConnection::from_matches(matches);
    let mut config = Config::read(matches)?.print_schema;

    if let Some(schema_name) = matches.get_one::<String>("schema") {
        config.schema = Some(schema_name.clone())
    }

    let filter = matches
        .get_many::<String>("table-name")
        .unwrap_or_default()
        .map(|table_name_regex| Regex::new(table_name_regex).map(Into::into))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("invalid argument for table filtering regex: {e}"));

    if matches.get_flag("only-tables") {
        config.filter = Filtering::OnlyTables(filter?)
    } else if matches.get_flag("except-tables") {
        config.filter = Filtering::ExceptTables(filter?)
    }

    if matches.get_flag("with-docs") {
        config.with_docs = DocConfig::DatabaseCommentsFallbackToAutoGeneratedDocComment;
    }

    if let Some(sorting) = matches.get_one::<String>("with-docs-config") {
        config.with_docs = sorting.parse()?;
    }

    if let Some(sorting) = matches.get_one::<String>("column-sorting") {
        match sorting as &str {
            "ordinal_position" => config.column_sorting = ColumnSorting::OrdinalPosition,
            "name" => config.column_sorting = ColumnSorting::Name,
            _ => return Err(format!("Invalid column sorting mode: {sorting}").into()),
        }
    }

    if let Some(path) = matches.get_one::<PathBuf>("patch-file") {
        config.patch_file = Some(path.clone());
    }

    if let Some(types) = matches.get_many("import-types") {
        let types = types.cloned().collect();
        config.import_types = Some(types);
    }

    if matches.get_flag("generate-custom-type-definitions") {
        config.generate_missing_sql_type_definitions = Some(false);
    }

    if let Some(derives) = matches.get_many("custom-type-derives") {
        let derives = derives.cloned().collect();
        config.custom_type_derives = Some(derives);
    }

    run_print_schema(&mut conn, &config, &mut stdout())?;
    Ok(())
}

fn regenerate_schema_if_file_specified(
    matches: &ArgMatches,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    use std::io::Read;

    let config = Config::read(matches)?;
    if let Some(ref path) = config.print_schema.file {
        let mut connection = InferConnection::from_matches(matches);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        if matches.get_flag("LOCKED_SCHEMA") {
            let mut buf = Vec::new();
            print_schema::run_print_schema(&mut connection, &config.print_schema, &mut buf)?;

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
            let schema = print_schema::output_schema(&mut connection, &config.print_schema)?;
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

        fs::File::create(toml_path.as_path()).unwrap();

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
