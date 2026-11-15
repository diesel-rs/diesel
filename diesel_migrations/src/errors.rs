//! Error types that represent migration errors.
//! These are split into multiple segments, depending on
//! where in the migration process an error occurs.

use std::error::Error;
use std::path::PathBuf;
use std::{fmt, io};

use diesel::migration::MigrationVersion;

use crate::file_based_migrations::DieselMigrationName;

/// Errors that occur while preparing to run migrations
#[derive(Debug)]
#[non_exhaustive]
pub enum MigrationError {
    /// The migration directory wasn't found
    MigrationDirectoryNotFound(PathBuf),
    /// Provided migration was in an unknown format
    UnknownMigrationFormat(PathBuf),
    /// General system IO error
    IoError(io::Error),
    /// Provided migration had an incompatible version number
    UnknownMigrationVersion(MigrationVersion<'static>),
    /// No migrations had to be/ could be run
    NoMigrationRun,
    /// `down.sql` file is missing, and you're asking for a `revert`
    /// or `redo`
    NoMigrationRevertFile,
}

impl Error for MigrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            MigrationError::IoError(ref error) => error.source(),
            _ => None,
        }
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            MigrationError::MigrationDirectoryNotFound(ref p) => write!(
                f,
                "Unable to find migrations directory in {p:?} or any parent directories."
            ),
            MigrationError::UnknownMigrationFormat(_) => write!(
                f,
                "Invalid migration directory: the directory's name should be \
                 <timestamp>_<name_of_migration>, and it should contain up.sql and \
                 optionally down.sql."
            ),
            MigrationError::IoError(ref error) => write!(f, "{error}"),
            MigrationError::UnknownMigrationVersion(ref version) => write!(
                f,
                "Unable to find migration version {version} to revert in the migrations directory."
            ),
            MigrationError::NoMigrationRun => write!(
                f,
                "No migrations have been run. Did you forget `diesel migration run`?"
            ),
            MigrationError::NoMigrationRevertFile => {
                write!(f, "Missing `down.sql` file to revert migration")
            }
        }
    }
}

impl PartialEq for MigrationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                &MigrationError::MigrationDirectoryNotFound(_),
                &MigrationError::MigrationDirectoryNotFound(_),
            ) => true,
            (
                MigrationError::UnknownMigrationFormat(p1),
                MigrationError::UnknownMigrationFormat(p2),
            ) => p1 == p2,
            _ => false,
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(e: io::Error) -> Self {
        MigrationError::IoError(e)
    }
}

/// Errors that occur while running migrations
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
#[non_exhaustive]
pub enum RunMigrationsError {
    /// A general migration error occurred
    MigrationError(DieselMigrationName, MigrationError),
    /// The provided migration included an invalid query
    QueryError(DieselMigrationName, diesel::result::Error),
    /// The provided migration was empty
    EmptyMigration(DieselMigrationName),
}

impl Error for RunMigrationsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RunMigrationsError::MigrationError(_, err) => Some(err),
            RunMigrationsError::QueryError(_, err) => Some(err),
            RunMigrationsError::EmptyMigration(_) => None,
        }
    }
}

impl fmt::Display for RunMigrationsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RunMigrationsError::MigrationError(v, _) => write!(f, "Failed to run {v}"),
            RunMigrationsError::QueryError(v, err) => {
                write!(f, "Failed to run {v}")?;

                if let diesel::result::Error::DatabaseError(_, error_info) = err {
                    let message = error_info.message();
                    if message.ends_with("cannot run inside a transaction block") {
                        write!(
                            f,
                            " (see https://docs.diesel.rs/{}.x/diesel_migrations/struct.FileBasedMigrations.html#transactions)",
                            env!("CARGO_PKG_VERSION")
                                .rsplit_once('.')
                                .expect("We get a semver version here")
                                .0
                        )?;
                    }
                }

                Ok(())
            }
            RunMigrationsError::EmptyMigration(v) => write!(
                f,
                "Failed to run {v} with: Attempted to run an empty migration."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MigrationError, RunMigrationsError};
    use crate::file_based_migrations::DieselMigrationName;
    use std::io;

    fn format_error(error: &dyn std::error::Error) -> String {
        std::iter::successors(Some(error), |error| error.source())
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join(": ")
    }

    fn migration_name() -> DieselMigrationName {
        DieselMigrationName::from_name("2024-01-01-000000_create_users").unwrap()
    }

    #[test]
    fn migration_error_forwards_the_source_of_io_errors() {
        let integer_error = u8::try_from(256_i32).unwrap_err();
        let io_error = io::Error::other(diesel::result::Error::IntegerConversion(integer_error));
        let error = MigrationError::from(io_error);

        assert_eq!(
            format_error(&error),
            format!("Internal integer conversion failed: {integer_error}")
        );
    }

    #[test]
    fn run_migrations_error_exposes_the_migration_error_as_source() {
        let error = RunMigrationsError::MigrationError(
            migration_name(),
            MigrationError::NoMigrationRevertFile,
        );

        assert_eq!(
            error.to_string(),
            "Failed to run 2024-01-01-000000_create_users"
        );
        assert_eq!(
            format_error(&error),
            "Failed to run 2024-01-01-000000_create_users: \
             Missing `down.sql` file to revert migration"
        );
    }

    #[test]
    fn run_migrations_error_exposes_the_query_error_as_source() {
        let error =
            RunMigrationsError::QueryError(migration_name(), diesel::result::Error::NotFound);

        assert_eq!(
            error.to_string(),
            "Failed to run 2024-01-01-000000_create_users"
        );
        assert_eq!(
            format_error(&error),
            "Failed to run 2024-01-01-000000_create_users: Record not found"
        );
    }

    #[test]
    fn run_migrations_error_keeps_the_transaction_hint() {
        let database_error = diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::Unknown,
            Box::new(String::from(
                "CREATE INDEX CONCURRENTLY cannot run inside a transaction block",
            )),
        );
        let error = RunMigrationsError::QueryError(migration_name(), database_error);
        let docs_version = env!("CARGO_PKG_VERSION").rsplit_once('.').unwrap().0;

        assert_eq!(
            format_error(&error),
            format!(
                "Failed to run 2024-01-01-000000_create_users \
                 (see https://docs.diesel.rs/{docs_version}.x/diesel_migrations/struct.FileBasedMigrations.html#transactions): \
                 CREATE INDEX CONCURRENTLY cannot run inside a transaction block"
            )
        );
    }
}
