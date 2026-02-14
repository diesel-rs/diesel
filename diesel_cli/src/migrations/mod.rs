use chrono::Utc;
use clap::{ArgAction, Args, Subcommand, ValueEnum};
use diesel::Connection;
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource};
use diesel_migrations::{FileBasedMigrations, HarnessWithOutput, MigrationError, MigrationHarness};
use fd_lock::RwLock;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::Display;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::{env, io};

use crate::database::InferConnection;
use crate::{config::Config, regenerate_schema_if_file_specified};

mod diff_schema;

#[derive(Debug, Args)]
pub struct MigrationArgs {
    #[command(subcommand)]
    command: MigrationCommand,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum MigrationFormat {
    Sql,
}

#[derive(Debug, Subcommand)]
pub enum MigrationCommand {
    /// Runs all pending migrations.
    Run,

    /// Reverts the specified migrations.
    Revert {
        /// Reverts previously run migration files.
        #[arg(id = "REVERT_ALL", long = "all", short = 'a', action = ArgAction::SetTrue, conflicts_with = "REVERT_NUMBER")]
        all: bool,

        /// Reverts the last `n` migration files.
        ///
        /// When this option is specified the last `n` migration files will be reverted. By default revert the last one.
        #[arg(
            id = "REVERT_NUMBER",
            long = "number",
            short = 'n',
            default_value = "1",
            conflicts_with = "REVERT_ALL"
        )]
        number: u64,
    },

    /// Reverts and re-runs the latest migration. Useful
    /// for testing that a migration can in fact be reverted.
    Redo {
        /// Reverts and re-runs all migrations.
        ///
        /// When this option is specified all migrations
        /// will be reverted and re-runs. Useful for testing
        /// that your migrations can be reverted and applied.
        #[arg(
            id = "REDO_ALL",
            long = "all",
            short = 'a',
            action = ArgAction::SetTrue,
            conflicts_with = "REDO_NUMBER"
        )]
        all: bool,

        /// Redo the last `n` migration files.
        ///
        /// When this option is specified the last `n` migration files
        /// will be reverted and re-runs. By default redo the last migration.
        #[arg(
            id = "REDO_NUMBER",
            long = "number",
            short = 'n',
            long_help = "When this option is specified the last `n` migration files will be reverted and re-runs. By default redo the last migration.",
            default_value = "1",
            conflicts_with = "REDO_ALL"
        )]
        number: u64,
    },

    /// Lists all available migrations, marking those that have been applied.
    List,

    /// Returns true if there are any pending migrations.
    Pending,

    /// Generate a new migration with the given name, and the current timestamp as the version.
    Generate {
        /// The name of the migration to create.
        #[arg(
            id = "MIGRATION_NAME",
            required = true,
            index = 1,
            required = true,
            num_args = 1
        )]
        migration_name: String,

        /// The version number to use when generating the migration.
        /// Defaults to the current timestamp, which should suffice
        /// for most use cases.
        #[arg(id = "MIGRATION_VERSION", long = "version", num_args = 1)]
        version: Option<String>,

        /// Don't generate a down.sql file.
        /// You won't be able to run migration `revert` or `redo`.
        #[arg(id = "MIGRATION_NO_DOWN_FILE", short = 'u', long = "no-down", action = ArgAction::SetTrue)]
        no_down: bool,

        /// The format of the migration to be generated.
        #[arg(
            id = "MIGRATION_FORMAT",
            long = "format",
            value_enum,
            default_value_t = MigrationFormat::Sql,
            num_args = 1
        )]
        format: MigrationFormat,

        /// Populate the generated migrations
        /// based on the current difference between
        /// your `schema.rs` file and the specified
        /// database.
        /// The generated migrations are not expected to
        /// be perfect. Be sure to check whether they meet
        /// your expectations. Adjust the generated output
        /// if that's not the case.
        #[arg(
            id = "SCHEMA_RS",
            long = "diff-schema",
            num_args = 0..=1,
            default_missing_value = "NOT_SET",
            require_equals = true,
        )]
        schema_rs: Option<String>,

        /// For SQLite 3.37 and above, detect `INTEGER PRIMARY KEY` columns as `BigInt`,
        /// when the table isn't declared with `WITHOUT ROWID`.
        /// See https://www.sqlite.org/lang_createtable.html#rowid for more information.
        /// Only used with the `--diff-schema` argument.
        #[arg(
            id = "SQLITE_INTEGER_PRIMARY_KEY_IS_BIGINT",
            long = "sqlite-integer-primary-key-is-bigint",
            requires = "SCHEMA_RS",
            action = ArgAction::SetTrue
        )]
        sqlite_integer_primary_key_is_bigint: bool,

        /// Table names to filter.
        #[arg(
            id = "TABLE_NAME",
            index = 2,
            num_args = 1..,
            action = ArgAction::Append
        )]
        table_name: Vec<String>,

        /// Only include tables from table-name that matches regexp.
        #[arg(
            id = "ONLY_TABLES",
            short = 'o',
            long = "only-tables",
            action = ArgAction::Append,
            num_args = 0,
            default_missing_value = "true",
            value_parser = clap::value_parser!(bool),
        )]
        only_tables: Vec<bool>,

        /// Exclude tables from table-name that matches regex.
        #[arg(
            id = "EXCEPT_TABLES",
            short = 'e',
            action = ArgAction::Append,
            num_args = 0,
            default_missing_value = "true",
            value_parser = clap::value_parser!(bool),
        )]
        except_tables: Vec<bool>,

        /// Select schema key from diesel.toml, use 'default' for print_schema without key.
        #[arg(
            id = "SCHEMA_KEY",
            long = "schema-key",
            action = clap::ArgAction::Append,
            default_values_t = vec!["default".to_string()]
        )]
        schema_key: Vec<String>,
    },
}

#[tracing::instrument]
pub(super) fn run_migration_command(
    args: MigrationArgs,
    database_url: Option<String>,
    config_file: Option<PathBuf>,
    locked_schema: bool,
    migration_dir: Option<PathBuf>,
) -> Result<(), crate::errors::Error> {
    match args.command {
        MigrationCommand::Run => {
            let (mut conn, dir) =
                conn_and_migration_dir(migration_dir, database_url.clone(), config_file.clone())?;

            run_migrations_with_output(&mut conn, dir)?;
            regenerate_schema_if_file_specified(config_file, database_url, locked_schema)?;
        }
        MigrationCommand::Revert { all, number } => {
            let (mut conn, dir) =
                conn_and_migration_dir(migration_dir, database_url.clone(), config_file.clone())?;

            if all {
                revert_all_migrations_with_output(&mut conn, dir)?;
            } else {
                for _ in 0..number {
                    match revert_migration_with_output(&mut conn, dir.clone()) {
                        Ok(_) => {}
                        Err(e) if e.is::<MigrationError>() => {
                            match e.downcast_ref::<MigrationError>() {
                                // If n is larger then the actual number of migrations,
                                // just stop reverting them
                                Some(MigrationError::NoMigrationRun) => break,
                                _ => return Err(crate::errors::Error::MigrationError(e)),
                            }
                        }
                        Err(e) => return Err(crate::errors::Error::MigrationError(e)),
                    }
                }
            }

            regenerate_schema_if_file_specified(config_file, database_url, locked_schema)?;
        }
        MigrationCommand::Redo { all, number } => {
            let (mut conn, dir) =
                conn_and_migration_dir(migration_dir, database_url.clone(), config_file.clone())?;
            redo_migrations(&mut conn, dir, all, number)?;
            regenerate_schema_if_file_specified(config_file, database_url, locked_schema)?;
        }
        MigrationCommand::List => {
            let (mut conn, dir) =
                conn_and_migration_dir(migration_dir, database_url.clone(), config_file.clone())?;

            list_migrations(&mut conn, dir)?;
        }
        MigrationCommand::Pending => {
            let (mut conn, dir) =
                conn_and_migration_dir(migration_dir, database_url.clone(), config_file.clone())?;

            let result = MigrationHarness::has_pending_migration(&mut conn, dir)
                .map_err(crate::errors::Error::MigrationError)?;
            println!("{result:?}");
        }
        MigrationCommand::Generate {
            migration_name,
            version,
            no_down,
            format,
            schema_rs,
            sqlite_integer_primary_key_is_bigint,
            table_name,
            only_tables,
            except_tables,
            schema_key,
        } => {
            let migrations_folder = migrations_dir(migration_dir, config_file.clone())?;
            let mut lock = RwLock::new(migration_folder_lock(migrations_folder.clone())?);
            let _ = lock.write().map_err(|err| {
                crate::errors::Error::FailedToAcquireMigrationFolderLock(
                    migrations_folder.clone(),
                    err.to_string(),
                )
            })?;
            let (up_sql, down_sql) = if let Some(schema_rs_arg) = schema_rs {
                let schema_key = schema_key
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "default".to_string());

                let config = Config::read(config_file.clone())?;
                let mut print_schema = config
                    .print_schema
                    .all_configs
                    .get(&schema_key)
                    .ok_or(crate::errors::Error::NoSchemaKeyFound(schema_key.clone()))?
                    .clone();

                if sqlite_integer_primary_key_is_bigint {
                    print_schema.sqlite_integer_primary_key_is_bigint = Some(true);
                }

                let diff_schema = if schema_rs_arg == "NOT_SET" {
                    print_schema
                        .file
                        .clone()
                        .ok_or(crate::errors::Error::NoSchemaKeyFound(schema_key))?
                } else {
                    PathBuf::from(schema_rs_arg)
                };
                self::diff_schema::generate_sql_based_on_diff_schema(
                    print_schema,
                    database_url,
                    &diff_schema,
                    table_name,
                    only_tables,
                    except_tables,
                )?
            } else {
                (String::new(), String::new())
            };

            let explicit_version = version.is_some();
            let migration_version = migration_version(version);
            let migration_dir = create_migration_dir(
                migrations_folder,
                &migration_name,
                migration_version,
                explicit_version,
            )?;

            match format {
                MigrationFormat::Sql => {
                    generate_sql_migration(&migration_dir, !no_down, up_sql, down_sql)?
                }
            }
        }
    }

    Ok(())
}

/// Creates a connection to the database and a migration directory
/// from the command line arguments.
///
/// See [migrations_dir] for more information on how the migration directory is found.
fn conn_and_migration_dir(
    migration_dir: Option<std::path::PathBuf>,
    database_url: Option<String>,
    config_file: Option<std::path::PathBuf>,
) -> Result<(InferConnection, FileBasedMigrations), crate::errors::Error> {
    let conn = InferConnection::from_maybe_url(database_url)?;
    let dir = migrations_dir(migration_dir, config_file)?;
    let dir = FileBasedMigrations::from_path(dir.clone())
        .map_err(|e| crate::errors::Error::from_migration_error(e, Some(dir)))?;

    Ok((conn, dir))
}

/// Opens the .diesel_lock file inside the migrations folder
/// Creates the file if it does not exist
/// A lock can be acquired on this file to make sure we don't have multiple instances of diesel
/// doing migration work
/// See [run_migration_command]::generate for an example
fn migration_folder_lock(dir: PathBuf) -> Result<File, crate::errors::Error> {
    let path = dir.join(".diesel_lock");
    match File::create_new(&path) {
        Ok(file) => Ok(file),
        Err(err) => {
            if matches!(err.kind(), io::ErrorKind::AlreadyExists) {
                File::open(&path).map_err(|err| crate::errors::Error::IoError(err, Some(path)))
            } else {
                Err(crate::errors::Error::IoError(err, Some(path)))
            }
        }
    }
}

fn create_migration_dir<'a>(
    migrations_dir: PathBuf,
    migration_name: &str,
    version: Box<dyn Display + 'a>,
    explicit_version: bool,
) -> Result<PathBuf, crate::errors::Error> {
    const MAX_MIGRATIONS_PER_SEC: u16 = u16::MAX;
    fn is_duplicate_version(full_version: &str, migration_folders: &Vec<PathBuf>) -> bool {
        for folder in migration_folders {
            if folder.to_string_lossy().starts_with(full_version) {
                return true;
            }
        }
        false
    }

    fn create(
        migrations_dir: &Path,
        version: &str,
        migration_name: &str,
    ) -> Result<PathBuf, crate::errors::Error> {
        let versioned_name = format!("{version}_{migration_name}");
        let path = migrations_dir.join(versioned_name);

        fs::create_dir(&path)
            .map_err(|e| crate::errors::Error::IoError(e, Some(path.to_path_buf())))?;
        Ok(path.to_path_buf())
    }

    let migration_folders: Vec<PathBuf> = migrations_dir
        .read_dir()
        .map_err(|err| crate::errors::Error::IoError(err, Some(migrations_dir.clone())))?
        .filter_map(|e| {
            if let Ok(e) = e
                && e.path().is_dir()
            {
                return Some(e.path().file_name()?.into());
            }
            None
        })
        .collect();

    // if there's an explicit version try to use it
    if explicit_version {
        let version = format!("{version}");
        if is_duplicate_version(&version, &migration_folders) {
            return Err(crate::errors::Error::DuplicateMigrationVersion(
                migrations_dir,
                version,
            ));
        }
        return create(&migrations_dir, &version, migration_name);
    }

    // else add a subversion so the versions stay unique
    for subversion in 0..=MAX_MIGRATIONS_PER_SEC {
        let full_version = format!("{version}-{subversion:04x}");
        if is_duplicate_version(&full_version, &migration_folders) {
            continue;
        }
        return create(&migrations_dir, &full_version, migration_name);
    }
    // if we get here it means the user is trying to generate > `MAX_MIGRATION_PER_SEC`
    // migrations per second
    Err(crate::errors::Error::TooManyMigrations(
        migrations_dir,
        version.to_string(),
    ))
}

fn generate_sql_migration(
    path: &Path,
    with_down: bool,
    up_sql: String,
    down_sql: String,
) -> Result<(), crate::errors::Error> {
    use std::io::Write;

    let migration_dir_relative = crate::convert_absolute_path_to_relative(
        path,
        &env::current_dir().map_err(|e| crate::errors::Error::IoError(e, None))?,
    );

    let up_path = path.join("up.sql");
    println!(
        "Creating {}",
        migration_dir_relative.join("up.sql").display()
    );
    let mut up = fs::File::create(&up_path)
        .map_err(|e| crate::errors::Error::IoError(e, Some(up_path.clone())))?;
    up.write_all(b"-- Your SQL goes here\n")
        .map_err(|e| crate::errors::Error::IoError(e, Some(up_path.clone())))?;
    up.write_all(up_sql.as_bytes())
        .map_err(|e| crate::errors::Error::IoError(e, Some(up_path.clone())))?;

    if with_down {
        let down_path = path.join("down.sql");
        println!(
            "Creating {}",
            migration_dir_relative.join("down.sql").display()
        );
        let mut down = fs::File::create(&down_path)
            .map_err(|e| crate::errors::Error::IoError(e, Some(down_path.clone())))?;
        down.write_all(b"-- This file should undo anything in `up.sql`\n")
            .map_err(|e| crate::errors::Error::IoError(e, Some(up_path.clone())))?;
        down.write_all(down_sql.as_bytes())
            .map_err(|e| crate::errors::Error::IoError(e, Some(up_path.clone())))?;
    }
    Ok(())
}

fn migration_version<'a>(matches: Option<String>) -> Box<dyn Display + 'a> {
    matches
        .map(|s| Box::new(s) as Box<dyn Display>)
        .unwrap_or_else(|| Box::new(Utc::now().format(crate::TIMESTAMP_FORMAT)))
}

pub fn run_migrations_with_output<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), crate::errors::Error>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    HarnessWithOutput::write_to_stdout(conn)
        .run_pending_migrations(migrations)
        .map(|_| ())
        .map_err(crate::errors::Error::MigrationError)
}

fn revert_all_migrations_with_output<Conn, DB>(
    conn: &mut Conn,
    migrations: FileBasedMigrations,
) -> Result<(), crate::errors::Error>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    HarnessWithOutput::write_to_stdout(conn)
        .revert_all_migrations(migrations)
        .map(|_| ())
        .map_err(crate::errors::Error::MigrationError)
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
) -> Result<(), crate::errors::Error>
where
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
    DB: Backend,
{
    let applied_migrations = conn
        .applied_migrations()
        .map_err(crate::errors::Error::MigrationError)?
        .into_iter()
        .collect::<HashSet<_>>();

    let mut migrations = MigrationSource::<DB>::migrations(&migrations)
        .map_err(crate::errors::Error::MigrationError)?;
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
pub fn migrations_dir(
    migration_dir: Option<std::path::PathBuf>,
    config_file: Option<std::path::PathBuf>,
) -> Result<PathBuf, crate::errors::Error> {
    if let Some(dir) = migration_dir {
        return Ok(dir);
    };

    let from_env_or_config = env::var("MIGRATION_DIRECTORY")
        .map(PathBuf::from)
        .ok()
        .map(Ok)
        .or_else(|| {
            Config::read(config_file)
                .map(|m| Some(m.migrations_directory?.dir))
                .transpose()
        });

    match from_env_or_config {
        Some(result) => result,
        None => FileBasedMigrations::find_migrations_directory()
            .map(|p| p.path().to_path_buf())
            .map_err(|e| crate::errors::Error::from_migration_error::<PathBuf>(e, None)),
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
    redo_all: bool,
    redo_number: u64,
) -> Result<(), crate::errors::Error>
where
    DB: Backend,
    Conn: MigrationHarness<DB> + Connection<Backend = DB> + 'static,
{
    let migrations = MigrationSource::<DB>::migrations(&migrations_dir)
        .map_err(crate::errors::Error::MigrationError)?
        .into_iter()
        .map(|m| (m.name().version().as_owned(), m))
        .collect::<HashMap<_, _>>();
    let applied_migrations = conn
        .applied_migrations()
        .map_err(crate::errors::Error::MigrationError)?;
    let versions_to_revert = if redo_all {
        &applied_migrations
    } else {
        let number = std::cmp::min(redo_number as usize, applied_migrations.len());
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
            let reverted_versions = if redo_all {
                harness.revert_all_migrations(migrations_dir.clone())?
            } else {
                (0..redo_number)
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
                .map_err(crate::errors::Error::MigrationError)?
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
            .map_err(crate::errors::Error::MigrationError)
    } else {
        migrations_inner(&mut HarnessWithOutput::write_to_stdout(conn))
            .map_err(crate::errors::Error::MigrationError)
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
