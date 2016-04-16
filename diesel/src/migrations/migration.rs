use connection::SimpleConnection;
use super::{MigrationError, RunMigrationsError};

use std::path::{Path, PathBuf};

pub trait Migration {
    fn version(&self) -> &str;
    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;
    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError>;
}

pub fn migration_from(path: PathBuf) -> Result<Box<Migration>, MigrationError> {
    if valid_sql_migration_directory(&path) {
        let version = try!(version_from_path(&path));
        Ok(Box::new(SqlFileMigration(path, version)))
    } else {
        Err(MigrationError::UnknownMigrationFormat(path))
    }
}

fn valid_sql_migration_directory(path: &Path) -> bool {
    file_names(path).map(|files| {
        files.contains(&"down.sql".into()) && files.contains(&"up.sql".into())
    }).unwrap_or(false)
}

fn file_names(path: &Path) -> Result<Vec<String>, MigrationError> {
    try!(path.read_dir()).map(|e| {
        Ok(try!(e).file_name().into_string().unwrap())
    }).filter(|name| match name {
        &Ok(ref n) => !n.starts_with("."),
        _ => true
    }).collect()
}

#[doc(hidden)]
pub fn version_from_path(path: &Path) -> Result<String, MigrationError> {
    path.file_name().unwrap()
        .to_string_lossy()
        .split("_")
        .nth(0)
        .map(|s| Ok(s.into()))
        .unwrap_or_else(|| {
            Err(MigrationError::UnknownMigrationFormat(path.to_path_buf()))
        })
}

use std::fs::File;
use std::io::Read;

struct SqlFileMigration(PathBuf, String);

impl Migration for SqlFileMigration {
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
    let mut file = try!(File::open(path));
    try!(file.read_to_string(&mut sql));
    try!(conn.batch_execute(&sql));
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::{version_from_path, valid_sql_migration_directory};

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
    fn files_begining_with_dot_are_allowed() {
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
