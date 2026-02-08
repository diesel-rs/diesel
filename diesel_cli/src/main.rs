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

use clap::{CommandFactory, FromArgMatches};

use database::InferConnection;
use diesel::Connection;
use diesel::backend::Backend;
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationHarness};
use similar_asserts::SimpleDiff;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::{Cli, DieselCliCommand};

use self::config::Config;
pub static TIMESTAMP_FORMAT: &str = "%Y-%m-%d-%H%M%S";

fn main() {
    if let Err(e) = inner_main() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}

fn inner_main() -> Result<(), crate::errors::Error> {
    let filter = EnvFilter::from_env("DIESEL_LOG");
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

    let cli = Cli::command();
    let matches = cli.get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap();

    let database_url = cli.database_url;
    let config_file = cli.config_file;
    let locked_schema = cli.locked_schema;

    match cli.command {
        DieselCliCommand::Migration(migration_args) => self::migrations::run_migration_command(
            migration_args,
            database_url,
            config_file,
            locked_schema,
        )?,
        DieselCliCommand::Setup {
            migration_dir,
            no_default_migration,
        } => self::database::run_setup_command(
            database_url,
            migration_dir,
            config_file,
            no_default_migration,
        )?,
        DieselCliCommand::Database(args) => {
            self::database::run_database_command(args, config_file, database_url, locked_schema)?
        }
        DieselCliCommand::Completions { shell } => self::cli::generate_completions_command(&shell),
        DieselCliCommand::PrintSchema(args) => {
            self::print_schema::run_infer_schema(&matches, args, config_file, database_url)?
        }
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

/// Checks if the migration directory exists, else creates it.
/// For more information see the `migrations_dir` function.
fn create_migrations_dir(
    migration_dir: Option<std::path::PathBuf>,
    config_file: Option<std::path::PathBuf>,
) -> Result<PathBuf, crate::errors::Error> {
    let dir = match self::migrations::migrations_dir(migration_dir, config_file) {
        Ok(dir) => dir,
        Err(_) => find_project_root()?.join("migrations"),
    };

    if dir.exists() {
        // This is a cleanup code for migrating from an
        // older version of diesel_cli that set a `.gitkeep` instead of a `.keep` file.
        // TODO: remove this after a few releases
        if let Ok(read_dir) = fs::read_dir(&dir)
            && let Some(dir_entry) =
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
    } else {
        create_migrations_directory(&dir)?;
    }

    Ok(dir)
}

fn create_config_file(
    config_file: Option<std::path::PathBuf>,
    migrations_dir: &Path,
) -> Result<(), crate::errors::Error> {
    use std::io::Write;
    let path = Config::file_path(config_file);
    if !path.exists() {
        let source_content = include_str!("default_files/diesel.toml").to_string();
        let migrations_dir_relative =
            convert_absolute_path_to_relative(migrations_dir, &find_project_root()?);
        // convert the path to a valid toml string (escaping backslashes on windows)
        let migrations_dir_toml_string = migrations_dir_relative
            .display()
            .to_string()
            .replace('\\', "\\\\");
        let modified_content = source_content.replace(
            "dir = \"migrations\"",
            &format!("dir = \"{migrations_dir_toml_string}\""),
        );
        let mut file = fs::File::create(&path)
            .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
        file.write_all(modified_content.as_bytes())
            .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
    }

    Ok(())
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
fn convert_absolute_path_to_relative(target_path: &Path, current_path: &Path) -> PathBuf {
    use std::path::Component;

    let abs_target = dunce::canonicalize(target_path).unwrap_or_else(|_| target_path.to_owned());
    let abs_current = dunce::canonicalize(current_path).unwrap_or_else(|_| current_path.to_owned());

    let mut target_components = abs_target.components();
    let mut current_components = abs_current.components();
    let mut components = Vec::new();
    loop {
        match (target_components.next(), current_components.next()) {
            (None, None) => {
                break;
            }
            (Some(target_component), None) => {
                components.push(target_component);
                components.extend(target_components);
                break;
            }
            (None, _) => {
                components.push(Component::ParentDir);
            }
            (Some(target_component), Some(current_component))
                if components.is_empty() && target_component == current_component => {}
            (Some(target_component), Some(Component::CurDir)) => {
                components.push(target_component);
            }
            (Some(target_component), Some(_)) => {
                components.push(Component::ParentDir);
                components.extend(current_components.map(|_| Component::ParentDir));
                components.push(target_component);
                components.extend(target_components);
                break;
            }
        }
    }
    components.iter().map(|c| c.as_os_str()).collect()
}

#[tracing::instrument]
fn regenerate_schema_if_file_specified(
    config_file: Option<std::path::PathBuf>,
    database_url: Option<String>,
    locked_schema: bool,
) -> Result<(), crate::errors::Error> {
    tracing::debug!("Regenerate schema if required");

    let config = Config::read(config_file)?.print_schema;
    for config in config.all_configs.values() {
        if let Some(ref path) = config.file {
            let mut connection = InferConnection::from_maybe_url(database_url.clone())?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| crate::errors::Error::IoError(e, Some(parent.to_owned())))?;
            }

            let schema = print_schema::output_schema(&mut connection, config)?;
            if locked_schema {
                let old_buf = std::fs::read_to_string(path)
                    .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;

                if schema.lines().ne(old_buf.lines()) {
                    let label = path.file_name().expect("We have a file name here");
                    let label = label.to_string_lossy();
                    println!(
                        "{}",
                        SimpleDiff::from_str(&old_buf, &schema, &label, "new schema")
                    );
                    return Err(crate::errors::Error::SchemaWouldChange(
                        path.display().to_string(),
                    ));
                }
            } else {
                std::fs::write(path, schema.as_bytes())
                    .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_owned())))?;
            }
        }
    }

    Ok(())
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
