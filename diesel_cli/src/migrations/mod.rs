use chrono::Utc;
use clap::ArgMatches;
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource};
use diesel::Connection;
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationError, MigrationHarness};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fs::{self};
use std::path::{Path, PathBuf};

use crate::database::InferConnection;
use crate::{config::Config, handle_error, regenerate_schema_if_file_specified};

mod diff_schema;

pub(super) fn run_migration_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    match matches.subcommand().unwrap() {
        ("run", _) => {
            let mut conn = InferConnection::from_matches(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            run_migrations_with_output(&mut conn, dir)?;
            regenerate_schema_if_file_specified(matches)?;
        }
        ("revert", args) => {
            let mut conn = InferConnection::from_matches(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            if args.get_flag("REVERT_ALL") {
                revert_all_migrations_with_output(&mut conn, dir)?;
            } else {
                let number = args.get_one::<u64>("REVERT_NUMBER").unwrap();
                for _ in 0..*number {
                    match revert_migration_with_output(&mut conn, dir.clone()) {
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
        ("redo", args) => {
            let mut conn = InferConnection::from_matches(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            redo_migrations(&mut conn, dir, args);
            regenerate_schema_if_file_specified(matches)?;
        }
        ("list", _) => {
            let mut conn = InferConnection::from_matches(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            list_migrations(&mut conn, dir)?;
        }
        ("pending", _) => {
            let mut conn = InferConnection::from_matches(matches);
            let dir = migrations_dir(matches).unwrap_or_else(handle_error);
            let dir = FileBasedMigrations::from_path(dir).unwrap_or_else(handle_error);
            let result = MigrationHarness::has_pending_migration(&mut conn, dir)?;
            println!("{result:?}");
        }
        ("generate", args) => {
            let migration_name = args.get_one::<String>("MIGRATION_NAME").unwrap();

            let (up_sql, down_sql) = if let Some(diff_schema) = args.get_one::<String>("SCHEMA_RS")
            {
                let config = Config::read(matches)?;
                let diff_schema = if diff_schema == "NOT_SET" {
                    config.print_schema.file.clone()
                } else {
                    Some(PathBuf::from(diff_schema))
                };
                if let Some(diff_schema) = diff_schema {
                    self::diff_schema::generate_sql_based_on_diff_schema(
                        config,
                        args,
                        &diff_schema,
                    )?
                } else {
                    (String::new(), String::new())
                }
            } else {
                (String::new(), String::new())
            };
            let version = migration_version(args);
            let versioned_name = format!("{version}_{migration_name}");
            let migration_dir = migrations_dir(matches)
                .unwrap_or_else(handle_error)
                .join(versioned_name);
            fs::create_dir(&migration_dir).unwrap();

            match args
                .get_one::<String>("MIGRATION_FORMAT")
                .map(|s| s as &str)
            {
                Some("sql") => generate_sql_migration(
                    &migration_dir,
                    !args.get_flag("MIGRATION_NO_DOWN_FILE"),
                    up_sql,
                    down_sql,
                ),
                Some(x) => return Err(format!("Unrecognized migration format `{x}`").into()),
                None => unreachable!("MIGRATION_FORMAT has a default value"),
            }
        }
        _ => unreachable!("The cli parser should prevent reaching here"),
    };

    Ok(())
}

fn generate_sql_migration(path: &Path, with_down: bool, up_sql: String, down_sql: String) {
    use std::io::Write;

    let migration_dir_relative =
        crate::convert_absolute_path_to_relative(path, &env::current_dir().unwrap());

    let up_path = path.join("up.sql");
    println!(
        "Creating {}",
        migration_dir_relative.join("up.sql").display()
    );
    let mut up = fs::File::create(up_path).unwrap();
    up.write_all(b"-- Your SQL goes here\n").unwrap();
    up.write_all(up_sql.as_bytes()).unwrap();

    if with_down {
        let down_path = path.join("down.sql");
        println!(
            "Creating {}",
            migration_dir_relative.join("down.sql").display()
        );
        let mut down = fs::File::create(down_path).unwrap();
        down.write_all(b"-- This file should undo anything in `up.sql`\n")
            .unwrap();
        down.write_all(down_sql.as_bytes()).unwrap();
    }
}

fn migration_version<'a>(matches: &'a ArgMatches) -> Box<dyn Display + 'a> {
    matches
        .get_one::<String>("MIGRATION_VERSION")
        .map(|s| Box::new(s) as Box<dyn Display>)
        .unwrap_or_else(|| Box::new(Utc::now().format(crate::TIMESTAMP_FORMAT)))
}

fn migrations_dir_from_cli(matches: &ArgMatches) -> Option<PathBuf> {
    matches.get_one("MIGRATION_DIRECTORY").cloned().or_else(|| {
        matches
            .subcommand()
            .and_then(|s| migrations_dir_from_cli(s.1))
    })
}

pub fn run_migrations_with_output<Conn, DB>(
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
        println!("  [{x}] {name}");
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
pub fn migrations_dir(matches: &ArgMatches) -> Result<PathBuf, MigrationError> {
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

/// Reverts all the migrations, and then runs them again, if the `--all`
/// argument is used. Otherwise it only redoes a specific number of migrations
/// if the `--number` argument is used.
/// We try to execute the migrations in a single transaction so that f either part fails,
/// the transaction is not committed.
/// If the list of migrations that need to be redone contains a single migration
/// with `run_in_transaction = false` or if the backend is MySQL we cannot use a
/// transaction.
fn redo_migrations<Conn, DB>(
    conn: &mut Conn,
    migrations_dir: FileBasedMigrations,
    args: &ArgMatches,
) where
    DB: Backend,
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
{
    let migrations = MigrationSource::<DB>::migrations(&migrations_dir)
        .unwrap_or_else(handle_error)
        .into_iter()
        .map(|m| (m.name().version().as_owned(), m))
        .collect::<HashMap<_, _>>();
    let applied_migrations = conn.applied_migrations().unwrap_or_else(handle_error);
    let versions_to_revert = if args.get_flag("REDO_ALL") {
        &applied_migrations
    } else {
        let number = args.get_one::<u64>("REDO_NUMBER").unwrap();
        let number = std::cmp::min(*number as usize, applied_migrations.len());
        &applied_migrations[..number]
    };
    let should_use_not_use_transaction = versions_to_revert.iter().any(|v| {
        migrations
            .get(v)
            .map(|m| !m.metadata().run_in_transaction())
            .unwrap_or_default()
    });

    let migrations_inner =
        |harness: &mut HarnessWithOutput<Conn, _>|
         -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            // revert all the migrations
            let reverted_versions = if args.get_flag("REDO_ALL") {
                harness.revert_all_migrations(migrations_dir.clone())?
            } else {
                let number = args.get_one::<u64>("REDO_NUMBER").unwrap();
                (0..*number)
                    .filter_map(|_| {
                        match harness.revert_last_migration(migrations_dir.clone()) {
                            Ok(v) => Some(Ok(v)),
                            Err(e) if e.is::<MigrationError>() => {
                                match e.downcast_ref::<MigrationError>() {
                                    // If n is larger then the actual number of migrations,
                                    // just stop reverting them
                                    Some(MigrationError::NoMigrationRun) => None,
                                    _ => Some(Err(e)),
                                }
                            }
                            Err(e) => Some(Err(e)),
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };

            // get a mapping between migrations and migration versions
             let mut migrations = MigrationSource::<DB>::migrations(&migrations_dir)
                 .unwrap_or_else(handle_error)
                 .into_iter()
                 .map(|m| (m.name().version().as_owned(), m))
                 .collect::<HashMap<_, _>>();

            // build a list of migrations that need to be applied
            let mut migrations = reverted_versions
                .into_iter()
                .map(|v| {
                    migrations
                        .remove(&v)
                        .ok_or_else(|| MigrationError::UnknownMigrationVersion(v.as_owned()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            // Sort the migrations by version to apply them in order.
            migrations.sort_by_key(|m| m.name().version().as_owned());

            // apply all outstanding migrations
            harness.run_migrations(&migrations)?;

            Ok(())
        };

    if !should_use_not_use_transaction && should_redo_migration_in_transaction(conn) {
        conn.transaction(|conn| migrations_inner(&mut HarnessWithOutput::write_to_stdout(conn)))
            .unwrap_or_else(handle_error);
    } else {
        migrations_inner(&mut HarnessWithOutput::write_to_stdout(conn))
            .unwrap_or_else(handle_error);
    }
}

#[cfg(feature = "mysql")]
fn should_redo_migration_in_transaction(t: &dyn Any) -> bool {
    !matches!(
        t.downcast_ref::<InferConnection>(),
        Some(InferConnection::Mysql(_))
    )
}

#[cfg(not(feature = "mysql"))]
fn should_redo_migration_in_transaction(_t: &dyn Any) -> bool {
    true
}
