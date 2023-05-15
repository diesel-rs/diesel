use std::fmt::Display;
use std::fs::{DirEntry, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use diesel::backend::Backend;
use diesel::connection::BoxableConnection;
use diesel::migration::{
    self, Migration, MigrationMetadata, MigrationName, MigrationSource, MigrationVersion,
};
use migrations_internals::TomlMetadata;

use crate::errors::{MigrationError, RunMigrationsError};

/// A migration source based on a migration directory in the file system
///
/// A valid migration directory contains a sub folder per migration.
/// Each migration folder contains a `up.sql` file containing the migration itself
/// and a `down.sql` file containing the necessary SQL to revert the migration.
/// Additionally each folder can contain a `metadata.toml` file controlling how the
/// individual migration should be handled by the migration harness.
///
/// To embed an existing migration folder into the final binary see
/// [`embed_migrations!`](crate::embed_migrations!).
///
/// ## Example
///
/// ```text
/// # Directory Structure
/// - 20151219180527_create_users
///     - up.sql
///     - down.sql
/// - 20160107082941_create_posts
///     - up.sql
///     - down.sql
///     - metadata.toml
/// ```
///
/// ```sql
/// -- 20151219180527_create_users/up.sql
/// CREATE TABLE users (
///   id SERIAL PRIMARY KEY,
///   name VARCHAR NOT NULL,
///   hair_color VARCHAR
/// );
/// ```
///
/// ```sql
/// -- 20151219180527_create_users/down.sql
/// DROP TABLE users;
/// ```
///
/// ```sql
/// -- 20160107082941_create_posts/up.sql
/// CREATE TABLE posts (
///   id SERIAL PRIMARY KEY,
///   user_id INTEGER NOT NULL,
///   title VARCHAR NOT NULL,
///   body TEXT
/// );
/// ```
///
/// ```sql
/// -- 20160107082941_create_posts/down.sql
/// DROP TABLE posts;
/// ```
///
/// ```toml
/// ## 20160107082941_create_posts/metadata.toml
///
/// ## specifies if a migration is executed inside a
/// ## transaction or not. This configuration is optional
/// ## by default all migrations are run in transactions.
/// ##
/// ## For certain types of migrations, like creating an
/// ## index onto a existing column, it is required
/// ## to set this to false
/// run_in_transaction = true
/// ```
#[derive(Clone)]
pub struct FileBasedMigrations {
    base_path: PathBuf,
}

impl FileBasedMigrations {
    /// Create a new file based migration source based on a specific path
    ///
    /// This methods fails if the path passed as argument is no valid migration directory
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, MigrationError> {
        for dir in migrations_directories(path.as_ref())? {
            let path = dir?.path();
            if !migrations_internals::valid_sql_migration_directory(&path) {
                return Err(MigrationError::UnknownMigrationFormat(path));
            }
        }
        Ok(Self {
            base_path: path.as_ref().to_path_buf(),
        })
    }

    /// Create a new file based migration source by searching the migration diretcory
    ///
    /// This method looks in the current and all parent directories for a folder named
    /// `migrations`
    ///
    /// This method fails if no valid migration directory is found
    pub fn find_migrations_directory() -> Result<Self, MigrationError> {
        Self::find_migrations_directory_in_path(std::env::current_dir()?.as_path())
    }

    /// Create a new file based migration source by searching a give path for the migration
    /// directory
    ///
    /// This method looks in the passed directory and all parent directories for a folder
    /// named `migrations`
    ///
    /// This method fails if no valid migration directory is found
    pub fn find_migrations_directory_in_path(
        path: impl AsRef<Path>,
    ) -> Result<Self, MigrationError> {
        let migrations_directory = search_for_migrations_directory(path.as_ref())?;
        Self::from_path(migrations_directory.as_path())
    }

    #[doc(hidden)]
    pub fn path(&self) -> &Path {
        &self.base_path
    }
}

fn search_for_migrations_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    migrations_internals::search_for_migrations_directory(path)
        .ok_or_else(|| MigrationError::MigrationDirectoryNotFound(path.to_path_buf()))
}

fn migrations_directories(
    path: &'_ Path,
) -> Result<impl Iterator<Item = Result<DirEntry, MigrationError>> + '_, MigrationError> {
    Ok(migrations_internals::migrations_directories(path)?.map(move |e| e.map_err(Into::into)))
}

fn migrations_in_directory(
    path: &'_ Path,
) -> Result<impl Iterator<Item = Result<SqlFileMigration, MigrationError>> + '_, MigrationError> {
    Ok(migrations_directories(path)?.map(|entry| SqlFileMigration::from_path(&entry?.path())))
}

impl<DB: Backend> MigrationSource<DB> for FileBasedMigrations {
    fn migrations(&self) -> migration::Result<Vec<Box<dyn Migration<DB>>>> {
        migrations_in_directory(&self.base_path)?
            .map(|r| Ok(Box::new(r?) as Box<dyn Migration<DB>>))
            .collect()
    }
}

struct SqlFileMigration {
    base_path: PathBuf,
    metadata: TomlMetadataWrapper,
    name: DieselMigrationName,
}

impl SqlFileMigration {
    fn from_path(path: &Path) -> Result<Self, MigrationError> {
        if migrations_internals::valid_sql_migration_directory(path) {
            let metadata = TomlMetadataWrapper(
                TomlMetadata::read_from_file(&path.join("metadata.toml")).unwrap_or_default(),
            );
            Ok(Self {
                base_path: path.to_path_buf(),
                metadata,
                name: DieselMigrationName::from_path(path)?,
            })
        } else {
            Err(MigrationError::UnknownMigrationFormat(path.to_path_buf()))
        }
    }
}

impl<DB: Backend> Migration<DB> for SqlFileMigration {
    fn run(&self, conn: &mut dyn BoxableConnection<DB>) -> migration::Result<()> {
        Ok(run_sql_from_file(
            conn,
            &self.base_path.join("up.sql"),
            &self.name,
        )?)
    }

    fn revert(&self, conn: &mut dyn BoxableConnection<DB>) -> migration::Result<()> {
        let down_path = self.base_path.join("down.sql");
        if matches!(down_path.metadata(), Err(e) if e.kind() == std::io::ErrorKind::NotFound) {
            Err(MigrationError::NoMigrationRevertFile.into())
        } else {
            Ok(run_sql_from_file(conn, &down_path, &self.name)?)
        }
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        &self.metadata
    }

    fn name(&self) -> &dyn MigrationName {
        &self.name
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DieselMigrationName {
    name: String,
    version: MigrationVersion<'static>,
}

impl Clone for DieselMigrationName {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            version: self.version.as_owned(),
        }
    }
}

impl DieselMigrationName {
    fn from_path(path: &Path) -> Result<Self, MigrationError> {
        let name = path
            .file_name()
            .ok_or_else(|| MigrationError::UnknownMigrationFormat(path.to_path_buf()))?
            .to_string_lossy();
        Self::from_name(&name)
    }

    pub(crate) fn from_name(name: &str) -> Result<Self, MigrationError> {
        let version = migrations_internals::version_from_string(name)
            .ok_or_else(|| MigrationError::UnknownMigrationFormat(PathBuf::from(name)))?;
        Ok(Self {
            name: name.to_owned(),
            version: MigrationVersion::from(version),
        })
    }
}

impl MigrationName for DieselMigrationName {
    fn version(&self) -> MigrationVersion {
        self.version.as_owned()
    }
}

impl Display for DieselMigrationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Default)]
#[doc(hidden)]
pub struct TomlMetadataWrapper(TomlMetadata);

impl TomlMetadataWrapper {
    #[doc(hidden)]
    pub const fn new(run_in_transaction: bool) -> Self {
        Self(TomlMetadata::new(run_in_transaction))
    }
}

impl MigrationMetadata for TomlMetadataWrapper {
    fn run_in_transaction(&self) -> bool {
        self.0.run_in_transaction
    }
}

fn run_sql_from_file<DB: Backend>(
    conn: &mut dyn BoxableConnection<DB>,
    path: &Path,
    name: &DieselMigrationName,
) -> Result<(), RunMigrationsError> {
    let map_io_err = |e| RunMigrationsError::MigrationError(name.clone(), MigrationError::from(e));

    let mut sql = String::new();
    let mut file = File::open(path).map_err(map_io_err)?;
    file.read_to_string(&mut sql).map_err(map_io_err)?;

    if sql.is_empty() {
        return Err(RunMigrationsError::EmptyMigration(name.clone()));
    }

    conn.batch_execute(&sql)
        .map_err(|e| RunMigrationsError::QueryError(name.clone(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use super::*;

    use self::tempfile::Builder;
    use std::fs;

    #[test]
    fn migration_directory_not_found_if_no_migration_dir_exists() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();

        assert_eq!(
            Err(MigrationError::MigrationDirectoryNotFound(
                dir.path().into()
            )),
            search_for_migrations_directory(dir.path())
        );
    }

    #[test]
    fn migration_directory_defaults_to_pwd_slash_migrations() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
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
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
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

    #[test]
    fn migration_paths_in_directory_ignores_files() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");
        let file_path = migrations_path.join("README.md");

        fs::create_dir(migrations_path.as_path()).unwrap();
        fs::File::create(file_path.as_path()).unwrap();

        let migrations = migrations_in_directory(&migrations_path)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(0, migrations.len());
    }

    #[test]
    fn migration_paths_in_directory_ignores_dot_directories() {
        let dir = Builder::new().prefix("diesel").tempdir().unwrap();
        let temp_path = dir.path().canonicalize().unwrap();
        let migrations_path = temp_path.join("migrations");
        let dot_path = migrations_path.join(".hidden_dir");

        fs::create_dir(migrations_path.as_path()).unwrap();
        fs::create_dir(dot_path.as_path()).unwrap();

        let migrations = migrations_in_directory(&migrations_path)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(0, migrations.len());
    }
}
