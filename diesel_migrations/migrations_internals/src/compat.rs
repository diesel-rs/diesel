use std::fmt;
use std::path::{Path, PathBuf};
use std::io::Write;

use diesel::migration::*;

use connection::*;
use context::*;
use directory::*;
use sql_plugin::*;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct MigrationName<'a> {
    pub migration: &'a Migration,
}

#[deprecated]
pub fn name(migration: &Migration) -> MigrationName {
    MigrationName {
        migration: migration,
    }
}

impl<'a> fmt::Display for MigrationName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.migration.name(), f)
    }
}

#[deprecated]
pub fn migration_from(path: PathBuf) -> Result<Box<Migration>, MigrationError> {
    SqlPlugin.load_migration_from_path(&path)
}

/// Runs all migrations that have not yet been run. This function will print all progress to
/// stdout. This function will return an `Err` if some error occurs reading the migrations, or if
/// any migration fails to run. Each migration is run in its own transaction, so some migrations
/// may be committed, even if a later migration fails to run.
///
/// It should be noted that this runs all migrations that have not already been run, regardless of
/// whether or not their version is later than the latest run migration. This is generally not a
/// problem, and eases the more common case of two developers generating independent migrations on
/// a branch. Whoever created the second one will eventually need to run the first when both
/// branches are merged.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
#[deprecated]
pub fn run_pending_migrations<Conn>(conn: &Conn) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
{
    MigrationContext::new(conn)?
        .with_source(MigrationsDirectory::locate()?)
        .run_pending_migrations()
}

/// Compares migrations found in `migrations_dir` to those that have been applied.
/// Returns a list of pathbufs and whether they have been applied.
#[deprecated]
pub fn mark_migrations_in_directory<Conn>(
    conn: &Conn,
    migrations_dir: &Path,
) -> Result<Vec<(Box<Migration>, bool)>, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    Ok(MigrationContext::new(conn)?
        .with_source(MigrationsDirectory::new(migrations_dir))
        .marked_migrations()?
        .map(|(m, run_on)| (m.into_inner(), run_on.is_some()))
        .collect())
}

// Returns true if there are outstanding migrations in the migrations directory, otherwise
// returns false. Returns an `Err` if there are problems with migration setup.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
#[deprecated]
pub fn any_pending_migrations<Conn>(conn: &Conn) -> Result<bool, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    MigrationContext::new(conn)?
        .with_source(MigrationsDirectory::locate()?)
        .has_pending_migrations()
}

/// Reverts the last migration that was run. Returns the version that was reverted. Returns an
/// `Err` if no migrations have ever been run.
///
/// See the [module level documentation](index.html) for information on how migrations should be
/// structured, and where Diesel will look for them by default.
#[deprecated]
pub fn revert_latest_migration<Conn>(conn: &Conn) -> Result<String, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    MigrationContext::new(conn)?
        .with_source(MigrationsDirectory::locate()?)
        .revert_last_migration()
}

#[deprecated]
pub fn revert_latest_migration_in_directory<Conn>(
    conn: &Conn,
    path: &Path,
) -> Result<String, RunMigrationsError>
where
    Conn: MigrationConnection,
{
    MigrationContext::new(conn)?
        .with_source(MigrationsDirectory::new(path))
        .revert_last_migration()
}

/// Run all pending migrations in the given list. Apps should likely be calling
/// `run_pending_migrations` or `run_pending_migrations_in_directory` instead.
#[deprecated]
pub fn run_migrations<Conn, List>(
    conn: &Conn,
    migrations: List,
    output: &mut Write,
) -> Result<(), RunMigrationsError>
where
    Conn: MigrationConnection,
    List: IntoIterator,
    List::Item: Migration,
{
    let mut context = MigrationContext::new(conn)?.with_output(output);
    let pending_migrations = context.mark_migrations(migrations)?.filter_map(|(m, run_on)| {
        if run_on.is_some() { None } else { Some(m) }
    });
    context.run_migrations(pending_migrations)?;
    Ok(())
}

/// Returns the directory containing migrations. Will look at for
/// $PWD/migrations. If it is not found, it will search the parents of the
/// current directory, until it reaches the root directory.  Returns
/// `MigrationError::MigrationDirectoryNotFound` if no directory is found.
#[deprecated]
pub fn find_migrations_directory() -> Result<PathBuf, MigrationError> {
    Ok(MigrationsDirectory::locate()?.path().into())
}

/// Searches for the migrations directory relative to the given path. See
/// `find_migrations_directory` for more details.
#[deprecated]
pub fn search_for_migrations_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    Ok(MigrationsDirectory::locate_relative_to(path)?.path().into())
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    extern crate tempdir;

    use super::search_for_migrations_directory;
    use diesel::migration::MigrationError;

    use self::tempdir::TempDir;
    use std::fs;

    #[test]
    fn migration_directory_not_found_if_no_migration_dir_exists() {
        let dir = TempDir::new("diesel").unwrap();

        assert_eq!(
            Err(MigrationError::MigrationDirectoryNotFound),
            search_for_migrations_directory(dir.path())
        );
    }

    #[test]
    fn migration_directory_defaults_to_pwd_slash_migrations() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");

        fs::create_dir(&migrations_path).unwrap();

        assert_eq!(
            Ok(migrations_path),
            search_for_migrations_directory(&temp_path)
        );
    }

    #[test]
    fn migration_directory_checks_parents() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");
        let child_path = temp_path.join("child");

        fs::create_dir(&child_path).unwrap();
        fs::create_dir(&migrations_path).unwrap();

        assert_eq!(
            Ok(migrations_path),
            search_for_migrations_directory(&child_path)
        );
    }
}
