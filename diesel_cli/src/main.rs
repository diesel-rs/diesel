// Built-in Lints
// Clippy lints
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
#![cfg_attr(not(test), warn(clippy::unwrap_used))]

mod config;

#[macro_use]
mod database;
mod cli;
mod errors;
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
use std::error::Error;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use self::config::Config;
pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

fn main() {
    if let Err(e) = inner_main() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}

fn inner_main() -> Result<(), crate::errors::Error> {
    let filter = EnvFilter::from_default_env();
    let fmt = tracing_subscriber::fmt::layer();

    tracing_subscriber::Registry::default()
        .with(filter)
        .with(fmt)
        .init();

    dotenvy::dotenv().map(|_| ()).or_else(|e| {
        if matches!(e, dotenvy::Error::Io(ref i) if i.kind() == std::io::ErrorKind::NotFound) {
            Ok(())
        } else {
            Err(e)
        }
    })?;

    let matches = cli::build_cli().get_matches();

    match matches
        .subcommand()
        .expect("Clap should prevent reaching this without subcommand")
    {
        ("migration", matches) => self::migrations::run_migration_command(matches)?,
        ("setup", matches) => run_setup_command(matches)?,
        ("database", matches) => run_database_command(matches)?,
        ("completions", matches) => generate_completions_command(matches),
        ("print-schema", matches) => run_infer_schema(matches)?,
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
    Ok(())
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

#[tracing::instrument]
fn run_setup_command(matches: &ArgMatches) -> Result<(), crate::errors::Error> {
    let migrations_dir = create_migrations_dir(matches)?;
    create_config_file(matches, &migrations_dir)?;

    database::setup_database(matches, &migrations_dir)?;
    Ok(())
}

/// Checks if the migration directory exists, else creates it.
/// For more information see the `migrations_dir` function.
fn create_migrations_dir(matches: &ArgMatches) -> Result<PathBuf, crate::errors::Error> {
    let dir = match self::migrations::migrations_dir(matches) {
        Ok(dir) => dir,
        Err(_) => find_project_root()?.join("migrations"),
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

fn create_config_file(
    matches: &ArgMatches,
    migrations_dir: &Path,
) -> Result<(), crate::errors::Error> {
    use std::io::Write;
    let path = Config::file_path(matches);
    if !path.exists() {
        let source_content = include_str!("default_files/diesel.toml").to_string();
        // convert the path to a valid toml string (escaping backslashes on windows)
        let migrations_dir_toml_string = migrations_dir.display().to_string().replace('\\', "\\\\");
        let modified_content = source_content.replace(
            "dir = \"migrations\"",
            &format!("dir = \"{}\"", migrations_dir_toml_string),
        );
        let mut file = fs::File::create(&path)
            .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
        file.write_all(modified_content.as_bytes())
            .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
    }

    Ok(())
}

#[tracing::instrument]
fn run_database_command(matches: &ArgMatches) -> Result<(), crate::errors::Error> {
    match matches
        .subcommand()
        .expect("Clap should prevent reaching this without subcommand")
    {
        ("setup", args) => {
            let migrations_dir = self::migrations::migrations_dir(matches)?;
            database::setup_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("reset", args) => {
            let migrations_dir = self::migrations::migrations_dir(matches)?;
            database::reset_database(args, &migrations_dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("drop", args) => database::drop_database_command(args)?,
        _ => unreachable!("The cli parser should prevent reaching here"),
    };
    Ok(())
}

#[tracing::instrument]
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
fn create_migrations_directory(path: &Path) -> Result<PathBuf, crate::errors::Error> {
    println!("Creating migrations directory at: {}", path.display());
    fs::create_dir_all(path)
        .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
    let keep_path = path.join(".keep");
    fs::File::create(&keep_path).map_err(|e| crate::errors::Error::IoError(e, Some(keep_path)))?;
    Ok(path.to_owned())
}

fn find_project_root() -> Result<PathBuf, crate::errors::Error> {
    let current_dir = env::current_dir().map_err(|e| crate::errors::Error::IoError(e, None))?;
    search_for_directory_containing_file(&current_dir, "diesel.toml")
        .or_else(|_| search_for_directory_containing_file(&current_dir, "Cargo.toml"))
}

/// Searches for the directory that holds the project's Cargo.toml, and returns
/// the path if it found it, or returns a `DatabaseError::ProjectRootNotFound`.
fn search_for_directory_containing_file(
    path: &Path,
    file: &str,
) -> Result<PathBuf, crate::errors::Error> {
    let toml_path = path.join(file);
    if toml_path.is_file() {
        Ok(path.to_owned())
    } else {
        path.parent()
            .map(|p| search_for_directory_containing_file(p, file))
            .unwrap_or_else(|| Err(crate::errors::Error::ProjectRootNotFound(path.into())))
            .map_err(|_| crate::errors::Error::ProjectRootNotFound(path.into()))
    }
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

    result.join(
        target_path
            .strip_prefix(current_path)
            .expect("Paths have the same base"),
    )
}

#[tracing::instrument]
fn run_infer_schema(matches: &ArgMatches) -> Result<(), crate::errors::Error> {
    use crate::print_schema::*;

    tracing::info!("Infer schema");
    let mut conn = InferConnection::from_matches(matches)?;
    let root_config = Config::read(matches)?
        .set_filter(matches)?
        .update_config(matches)?
        .print_schema;
    for config in root_config.all_configs.values() {
        run_print_schema(&mut conn, config, &mut stdout())?;
    }

    Ok(())
}

#[tracing::instrument]
fn regenerate_schema_if_file_specified(matches: &ArgMatches) -> Result<(), crate::errors::Error> {
    tracing::debug!("Regenerate schema if required");

    let config = Config::read(matches)?.print_schema;
    for config in config.all_configs.values() {
        if let Some(ref path) = config.file {
            let mut connection = InferConnection::from_matches(matches)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| crate::errors::Error::IoError(e, Some(parent.to_owned())))?;
            }

            if matches.get_flag("LOCKED_SCHEMA") {
                let mut buf = Vec::new();
                print_schema::run_print_schema(&mut connection, config, &mut buf)?;

                let old_buf = std::fs::read(path)
                    .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;

                if buf != old_buf {
                    return Err(crate::errors::Error::SchemaWouldChange(
                        path.display().to_string(),
                    ));
                }
            } else {
                let schema = print_schema::output_schema(&mut connection, config)?;
                std::fs::write(path, schema.as_bytes())
                    .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
            }
        }
    }

    Ok(())
}

fn supported_backends() -> String {
    let features = &[
        #[cfg(feature = "postgres")]
        "postgres",
        #[cfg(feature = "mysql")]
        "mysql",
        #[cfg(feature = "sqlite")]
        "sqlite",
    ];

    features.join(" ")
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

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

        let res = search_for_directory_containing_file(&temp_path, "Cargo.toml");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), temp_path);
    }

    #[test]
    fn cargo_toml_not_found_if_no_cargo_toml() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
        let temp_path = dir.path().canonicalize().unwrap();

        assert!(matches!(
            search_for_directory_containing_file(&temp_path, "Cargo.toml"),
            Err(crate::errors::Error::ProjectRootNotFound(p)) if p == temp_path,
        ));
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
