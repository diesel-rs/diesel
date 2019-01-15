use diesel::connection::SimpleConnection;
use diesel::migration::*;

use std::fmt;
use std::path::{Path, PathBuf};

#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct MigrationName<'a> {
    pub migration: &'a Migration,
}

pub fn name(migration: &Migration) -> MigrationName {
    MigrationName { migration }
}

impl<'a> fmt::Display for MigrationName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let file_name = self
            .migration
            .file_path()
            .and_then(|file_path| file_path.file_name())
            .and_then(|file| file.to_str());
        if let Some(name) = file_name {
            f.write_str(name)?;
        } else {
            f.write_str(self.migration.version())?;
        }
        Ok(())
    }
}

#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
pub struct MigrationFileName<'a> {
    pub migration: &'a Migration,
    pub sql_file: &'a str,
}

pub fn file_name<'a>(migration: &'a Migration, sql_file: &'a str) -> MigrationFileName<'a> {
    MigrationFileName {
        migration,
        sql_file,
    }
}

impl<'a> fmt::Display for MigrationFileName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fpath = match self.migration.file_path() {
            None => return Err(fmt::Error),
            Some(v) => v.join(self.sql_file),
        };
        f.write_str(fpath.to_str().unwrap_or("Invalid utf8 in filename"))?;
        Ok(())
    }
}

pub fn migration_from(path: PathBuf) -> Result<Box<Migration>, MigrationError> {
    #[cfg(feature = "barrel")]
    match ::barrel::integrations::diesel::migration_from(&path) {
        Some(migration) => return Ok(migration),
        None => {}
    }

    if valid_sql_migration_directory(&path) {
        let version = version_from_path(&path)?;
        Ok(Box::new(SqlFileMigration(path, version)))
    } else {
        Err(MigrationError::UnknownMigrationFormat(path))
    }
}

fn valid_sql_migration_directory(path: &Path) -> bool {
    file_names(path)
        .map(|files| files.contains(&"down.sql".into()) && files.contains(&"up.sql".into()))
        .unwrap_or(false)
}

fn file_names(path: &Path) -> Result<Vec<String>, MigrationError> {
    path.read_dir()?
        .map(|entry| {
            let file_name = entry?.file_name();

            // FIXME(killercup): Decide whether to add MigrationError variant for this
            match file_name.into_string() {
                Ok(utf8_file_name) => Ok(utf8_file_name),
                Err(original_os_string) => panic!(
                    "Can't convert file name `{:?}` into UTF8 string",
                    original_os_string
                ),
            }
        })
        .filter(|file_name| match *file_name {
            Ok(ref name) => !name.starts_with('.'),
            _ => true,
        })
        .collect()
}

#[doc(hidden)]
pub fn version_from_path(path: &Path) -> Result<String, MigrationError> {
    path.file_name()
        .unwrap_or_else(|| panic!("Can't get file name from path `{:?}`", path))
        .to_string_lossy()
        .split('_')
        .nth(0)
        .map(|s| Ok(s.replace('-', "")))
        .unwrap_or_else(|| Err(MigrationError::UnknownMigrationFormat(path.to_path_buf())))
}

use std::fs::File;
use std::io::Read;

struct SqlFileMigration(PathBuf, String);

impl Migration for SqlFileMigration {
    fn file_path(&self) -> Option<&Path> {
        Some(self.0.as_path())
    }

    fn version(&self) -> &str {
        &self.1
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        run_sql_from_file(conn, &self.0.join("up.sql"))
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        run_sql_from_file(conn, &self.0.join("down.sql"))
    }
}

fn run_sql_from_file(conn: &SimpleConnection, path: &Path) -> Result<(), RunMigrationsError> {
    let mut sql = String::new();
    let mut file = File::open(path)?;
    file.read_to_string(&mut sql)?;

    if sql.is_empty() {
        return Err(RunMigrationsError::EmptyMigration);
    }

    conn.batch_execute(&sql)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::{valid_sql_migration_directory, version_from_path};

    use self::tempdir::TempDir;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn files_are_not_valid_sql_file_migrations() {
        let dir = TempDir::new("diesel").unwrap();
        let file_path = dir.path().join("12345");

        fs::File::create(&file_path).unwrap();

        assert!(!valid_sql_migration_directory(&file_path));
    }

    #[test]
    fn directory_containing_exactly_up_sql_and_down_sql_is_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();

        assert!(valid_sql_migration_directory(&folder));
    }

    #[test]
    fn directory_containing_unknown_files_is_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();
        fs::File::create(folder.join("foo")).unwrap();

        assert!(valid_sql_migration_directory(&folder));
    }

    #[test]
    fn files_beginning_with_dot_are_allowed() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();
        fs::File::create(folder.join(".foo")).unwrap();

        assert!(valid_sql_migration_directory(&folder));
    }

    #[test]
    fn empty_directory_is_not_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();

        assert!(!valid_sql_migration_directory(&folder));
    }

    #[test]
    fn directory_with_only_up_sql_is_not_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();

        assert!(!valid_sql_migration_directory(&folder));
    }

    #[test]
    fn migration_version_is_based_on_folder_name() {
        let path = PathBuf::new().join("migrations").join("12345");

        assert_eq!(Ok("12345".into()), version_from_path(&path));
    }

    #[test]
    fn migration_version_allows_additional_naming() {
        let path = PathBuf::new().join("migrations").join("54321_create_stuff");

        assert_eq!(Ok("54321".into()), version_from_path(&path));
    }

    #[test]
    fn migration_version_when_dir_doesnt_start_with_num_is_allowed() {
        let path = PathBuf::new().join("migrations").join("create_stuff_12345");

        assert_eq!(Ok("create".into()), version_from_path(&path));
    }
}
