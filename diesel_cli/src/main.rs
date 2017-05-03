// Built-in Lints
#![deny(warnings, missing_copy_implementations)]

// Clippy lints
#![cfg_attr(feature = "clippy", allow(unstable_features))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file="../clippy.toml")))]
#![cfg_attr(feature = "clippy", allow(
    option_map_unwrap_or_else, option_map_unwrap_or,
))]
#![cfg_attr(feature = "clippy", warn(
    wrong_pub_self_convention, mut_mut, non_ascii_literal, similar_names, unicode_not_nfc,
    if_not_else, items_after_statements, used_underscore_binding,
))]
#![cfg_attr(all(test, feature = "clippy"), allow(result_unwrap_used))]

extern crate chrono;
extern crate clap;
#[cfg_attr(any(feature="mysql", feature="postgres"), macro_use)]
extern crate diesel;
extern crate dotenv;
extern crate diesel_infer_schema;

mod database_error;
#[macro_use]
mod database;
mod cli;
mod pretty_printing;

use chrono::*;
use clap::{ArgMatches,Shell};
use diesel::migrations::schema::*;
use diesel::types::{FromSql, VarChar};
use diesel::{migrations, Connection, Insertable};
use std::any::Any;
use std::error::Error;
use std::io::stdout;
use std::path::{PathBuf, Path};
use std::{env, fs};

use self::database_error::{DatabaseError, DatabaseResult};
use diesel::migrations::TIMESTAMP_FORMAT;


fn main() {
    use self::dotenv::dotenv;
    dotenv().ok();

    let matches = cli::build_cli().get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches),
        ("setup", Some(matches)) => run_setup_command(matches),
        ("database", Some(matches)) => run_database_command(matches),
        ("bash-completion", Some(matches)) => generate_bash_completion_command(matches),
        ("print-schema", Some(matches)) => run_infer_schema(matches),
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn run_migration_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("run", Some(args)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(args);
            call_with_conn!(database_url, migrations::run_pending_migrations_in_directory(&dir, &mut stdout()))
                .unwrap_or_else(handle_error);
        }
        ("revert", Some(args)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(args);
            call_with_conn!(database_url, migrations::revert_latest_migration_in_directory(&dir))
                .unwrap_or_else(handle_error);
        }
        ("redo", Some(args)) => {
            let database_url = database::database_url(matches);
            let dir = migrations_dir(args);
            call_with_conn!(database_url, redo_latest_migration(&dir));
        }
        ("list", Some(args)) => {
            use std::cmp::Ordering;
            use std::ffi::OsStr;

            let database_url = database::database_url(matches);
            let dir = migrations_dir(args);
            let migrations = call_with_conn!(database_url, migrations::mark_migrations_in_directory(&dir))
                .unwrap_or_else(handle_error);

            // Since our migrations came from `mark_migrations_in_directory` they should all have valid file names.
            let mut sorted = migrations.into_iter().map(|(path_buf, applied)| {
                let path_buf = path_buf.expect("Found migration with invalid file name");
                let file_path = path_buf.as_path();
                let file_name = file_path.file_name()
                    .and_then(OsStr::to_str)
                    .map(str::to_string)
                    .expect(&format!("Error getting file name from {:?}", path_buf));
                (file_name, applied)
            }).collect::<Vec<_>>();

            // sort migrations by version timestamp, push non-conforming timestamped versions to the bottom
            sorted.sort_by(|a, b| {
                let version_a = a.0.split('_').nth(0).expect(&format!("Unexpected version format {:?}", a.0));
                let ver_date_a = UTC.datetime_from_str(version_a, TIMESTAMP_FORMAT);
                let version_b = b.0.split('_').nth(0).expect(&format!("Unexpected version format {:?}", b.0));
                let ver_date_b = UTC.datetime_from_str(version_b, TIMESTAMP_FORMAT);
                match (ver_date_a.is_ok(), ver_date_b.is_ok()) {
                    (false, false) => version_a.to_lowercase().cmp(&version_b.to_lowercase()),
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (true, true) => {
                        let a = ver_date_a.unwrap();
                        let b = ver_date_b.unwrap();
                        a.cmp(&b)
                    }
                }
            });

            println!("Migrations:");
            for (name, applied) in sorted {
                let x = if applied { 'X' } else { ' ' };
                println!("  [{}] {}", x, name);
            }
        }
        ("generate", Some(args)) => {
            use std::io::Write;

            let migration_name = args.value_of("MIGRATION_NAME").unwrap();
            let version = migration_version(args);
            let versioned_name = format!("{}_{}", version, migration_name);
            let migration_dir = migrations_dir(args).join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            let migration_dir_relative = convert_absolute_path_to_relative(
                &migration_dir,
                &env::current_dir().unwrap()
            );

            let up_path = migration_dir.join("up.sql");
            println!("Creating {}", migration_dir_relative.join("up.sql").display());
            let mut up = fs::File::create(up_path).unwrap();
            up.write_all(b"-- Your SQL goes here").unwrap();

            let down_path = migration_dir.join("down.sql");
            println!("Creating {}", migration_dir_relative.join("down.sql").display());
            let mut down = fs::File::create(down_path).unwrap();
            down.write_all(b"-- This file should undo anything in `up.sql`").unwrap();
        }
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

use std::fmt::Display;
fn migration_version<'a>(matches: &'a ArgMatches) -> Box<Display + 'a> {
    matches.value_of("MIGRATION_VERSION").map(|s| Box::new(s) as Box<Display>)
        .unwrap_or_else(|| Box::new(UTC::now().format(TIMESTAMP_FORMAT)))
}

fn migrations_dir(matches: &ArgMatches) -> PathBuf {
    matches.value_of("MIGRATION_DIRECTORY")
        .map(PathBuf::from)
        .or_else(|| {
            env::var("MIGRATION_DIRECTORY").map(PathBuf::from).ok()
        }).unwrap_or_else(|| {
            migrations::find_migrations_directory()
                .unwrap_or_else(handle_error)
        })
}

fn run_setup_command(matches: &ArgMatches) {
    migrations::find_migrations_directory()
        .unwrap_or_else(|_| {
            create_migrations_directory()
                .unwrap_or_else(handle_error)
        });

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

fn generate_bash_completion_command(_: &ArgMatches) {
    cli::build_cli().gen_completions_to("diesel", Shell::Bash, &mut stdout());
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
fn redo_latest_migration<Conn>(conn: &Conn, migrations_dir: &Path) where
        Conn: Connection + Any,
        String: FromSql<VarChar, Conn::Backend>,
        for<'a> &'a NewMigration<'a>:
            Insertable<__diesel_schema_migrations::table, Conn::Backend>,
{
    let migration_inner = || {
        let reverted_version = try!(migrations::revert_latest_migration_in_directory(conn, migrations_dir));
        migrations::run_migration_with_version(conn, migrations_dir, &reverted_version, &mut stdout())
    };
    if should_redo_migration_in_transaction(conn) {
        conn.transaction(migration_inner).unwrap_or_else(handle_error);
    } else {
        migration_inner().unwrap_or_else(handle_error);
    }
}

#[cfg(feature="mysql")]
fn should_redo_migration_in_transaction(t: &Any) -> bool {
    !t.is::<::diesel::mysql::MysqlConnection>()
}

#[cfg(not(feature="mysql"))]
fn should_redo_migration_in_transaction(_t: &Any) -> bool {
    true
}

#[cfg_attr(feature="clippy", allow(needless_pass_by_value))]
fn handle_error<E: Error, T>(error: E) -> T {
    println!("{}", error);
    ::std::process::exit(1);
}

// Converts an absolute path to a relative path, with the restriction that the
// target path must be in the same directory or above the current path.
fn convert_absolute_path_to_relative(target_path: &Path, mut current_path: &Path)
    -> PathBuf
{
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

fn run_infer_schema(matches: &ArgMatches) {
    let database_url = database::database_url(matches);
    let schema_name = matches.value_of("schema");

    let filtering_tables = matches.values_of("table-name").map(|values| {
        values.map(|table_name| {
            if cfg!(not(feature = "postgres")) || table_name.contains('.') {
                String::from(table_name)
            } else {
                format!("{}.{}", schema_name.unwrap_or("public"), table_name)
            }
        }).collect()
    }).unwrap_or_else(::std::collections::HashSet::new);

    let is_whitelist = matches.is_present("whitelist");
    let is_blacklist = matches.is_present("blacklist");

    let table_names = diesel_infer_schema::load_table_names(&database_url, schema_name)
        .expect(&format!("Could not load table names from database `{}`{}",
            database_url,
            if let Some(name) = schema_name {
                format!(" with schema `{}`", name)
            } else {
                "".into()
            }
        ));

    let tables = table_names.iter()
        .filter_map(|table| {
            let table_name = table.to_string();
            if is_whitelist && !filtering_tables.contains(&table_name[..]) {
                return None;
            }
            if is_blacklist && filtering_tables.contains(&table_name[..]) {
                return None;
            }
            Some(diesel_infer_schema::expand_infer_table_from_schema(&database_url, table)
                .expect(&format!("Could not load table `{}`", table.to_string())))
        });

    let schema = diesel_infer_schema::handle_schema(tables, schema_name);

    let pretty = pretty_printing::format_schema(schema.as_str())
        .expect("Could not write to stdout");

    println!("{}", pretty);
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
                   convert_absolute_path_to_relative(&PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                     &PathBuf::from("projects/foo")));
        assert_eq!(PathBuf::from("../migrations/12345_create_user"),
                   convert_absolute_path_to_relative(&PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                     &PathBuf::from("projects/foo/src")));
        assert_eq!(PathBuf::from("../../../migrations/12345_create_user"),
                   convert_absolute_path_to_relative(&PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                     &PathBuf::from("projects/foo/src/controllers/errors")));
        assert_eq!(PathBuf::from("12345_create_user"),
                   convert_absolute_path_to_relative(&PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                     &PathBuf::from("projects/foo/migrations")));
        assert_eq!(PathBuf::from("../12345_create_user"),
                   convert_absolute_path_to_relative(&PathBuf::from("projects/foo/migrations/12345_create_user"),
                                                     &PathBuf::from("projects/foo/migrations/67890_create_post")));
    }
}
