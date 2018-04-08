use std::path::{Path, PathBuf};
use std::env;

use diesel::migration::{
    MigrationSource, MigrationError, Migration, MigrationsDirectoryPlugin, AnnotatedMigration
};
use super::sql_plugin::SqlPlugin;

#[derive(Debug)]
pub struct MigrationsDirectory {
    path: PathBuf,
    plugins: Vec<Box<MigrationsDirectoryPlugin>>,
}

impl MigrationsDirectory {
    /// Use a known directory as the migrations directory
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let mut result = MigrationsDirectory {
            path: path.into(),
            plugins: Vec::new()
        };
        result.enable_plugin(SqlPlugin);
        result
    }

    /// Find a migrations directory in the specified path or one of its parents
    pub fn locate_relative_to(path: &Path) -> Result<Self, MigrationError> {
        let migration_path = path.join("migrations");
        if migration_path.is_dir() {
            Ok(MigrationsDirectory::new(migration_path))
        } else {
            path.parent()
                .ok_or(MigrationError::MigrationDirectoryNotFound)
                .and_then(MigrationsDirectory::locate_relative_to)
        }
    }

    /// Find the migrations directory relative to the current directory
    pub fn locate() -> Result<Self, MigrationError> {
        env::current_dir()
            .map_err(|_| MigrationError::MigrationDirectoryNotFound)
            .and_then(|path| MigrationsDirectory::locate_relative_to(&path))
    }

    /// Enable a plugin to use when loading migrations from this directory
    pub fn enable_plugin<T: MigrationsDirectoryPlugin>(&mut self, plugin: T) {
        self.plugins.push(Box::new(plugin));
    }

    /// Default way to get a version number for a migration
    pub fn version_from_path(path: &Path) -> Result<String, MigrationError> {
        path.file_name()
            .expect(&format!("Can't get file name from path `{:?}`", path))
            .to_string_lossy()
            .split('_')
            .nth(0)
            .map(|s| Ok(s.replace('-', "")))
            .unwrap_or_else(|| Err(MigrationError::UnknownMigrationFormat(path.into())))
    }

    /// Default way to get the name for a migration
    pub fn name_from_path(path: &Path) -> Result<String, MigrationError> {
        Ok(path.file_name()
            .expect(&format!("Can't get file name from path `{:?}`", path))
            .to_string_lossy()
            .into())
    }

    fn migration_from_path_inner(&self, path: &Path) -> Result<Box<Migration>, MigrationError> {
        // Try each of the plugins starting from the most recently added (to allow overriding)
        for plugin in self.plugins.iter().rev() {
            match plugin.load_migration_from_path(path) {
                Err(MigrationError::UnknownMigrationFormat(_)) => continue,
                other => return other,
            }
        }
        Err(MigrationError::UnknownMigrationFormat(path.into()))
    }

    /// Load a migration from a path
    pub fn migration_from_path(&self, path: &Path) -> Result<AnnotatedMigration, MigrationError> {
        let mut result = AnnotatedMigration::new(self.migration_from_path_inner(path)?);
        for plugin in self.plugins.iter() {
            plugin.load_annotations_from_path(path, &mut result)?;
        }
        Ok(result)
    }

    /// Get the path to this directory
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl MigrationSource for MigrationsDirectory {
    type MigrationEntry = AnnotatedMigration;

    /// List all migrations in the directory
    fn list_migrations(&self) -> Result<Vec<Self::MigrationEntry>, MigrationError> {
        try!(self.path.read_dir())
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => return Some(Err(e.into())),
                };
                if entry.file_name().to_string_lossy().starts_with('.') {
                    None
                } else {
                    Some(self.migration_from_path(&entry.path()))
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::MigrationsDirectory;
    use diesel::migration::MigrationError;

    use self::tempdir::TempDir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn migration_version_is_based_on_folder_name() {
        let path = PathBuf::new().join("migrations").join("12345");

        assert_eq!(Ok("12345".into()), MigrationsDirectory::version_from_path(&path));
    }

    #[test]
    fn migration_version_allows_additional_naming() {
        let path = PathBuf::new().join("migrations").join("54321_create_stuff");

        assert_eq!(Ok("54321".into()), MigrationsDirectory::version_from_path(&path));
    }

    #[test]
    fn migration_version_when_dir_doesnt_start_with_num_is_allowed() {
        let path = PathBuf::new().join("migrations").join("create_stuff_12345");

        assert_eq!(Ok("create".into()), MigrationsDirectory::version_from_path(&path));
    }

    #[test]
    fn migration_directory_not_found_if_no_migration_dir_exists() {
        let dir = TempDir::new("diesel").unwrap();

        assert_eq!(
            Err(MigrationError::MigrationDirectoryNotFound),
            MigrationsDirectory::locate_relative_to(dir.path()).map(|m| m.path().to_owned())
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
            MigrationsDirectory::locate_relative_to(&temp_path).map(|m| m.path().to_owned())
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
            MigrationsDirectory::locate_relative_to(&child_path).map(|m| m.path().to_owned())
        );
    }
}
