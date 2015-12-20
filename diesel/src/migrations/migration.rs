use connection::Connection;
use super::{MigrationError, RunMigrationsError};

use std::path::{Path, PathBuf};

pub trait Migration {
    fn version(&self) -> String;
    fn run(&self, conn: &Connection) -> Result<(), RunMigrationsError>;
    fn revert(&self, conn: &Connection) -> Result<(), RunMigrationsError>;
}

pub fn migration_from(path: PathBuf) -> Result<Box<Migration>, MigrationError> {
    if try!(valid_sql_migration_directory(&path)) {
        Ok(Box::new(SqlFileMigration(path)))
    } else {
        Err(MigrationError::UnknownMigrationFormat(path))
    }
}

fn valid_sql_migration_directory(path: &Path) -> Result<bool, MigrationError> {
    for entry in try!(path.read_dir()) {
        let entry = try!(entry);
        let file_name = entry.file_name();
        if &file_name != "up.sql" && &file_name != "down.sql" {
            return Ok(false);
        }
    }
    Ok(true)
}

use std::fs::File;
use std::io::Read;

struct SqlFileMigration(PathBuf);

impl Migration for SqlFileMigration {
    fn version(&self) -> String {
        self.0.components().last().unwrap()
            .as_ref()
            .to_os_string()
            .into_string()
            .unwrap()
    }

    fn run(&self, conn: &Connection) -> Result<(), RunMigrationsError> {
        println!("Running migration {}", self.version());
        run_sql_from_file(conn, &self.0.join("up.sql"))
    }

    fn revert(&self, conn: &Connection) -> Result<(), RunMigrationsError> {
        println!("Reverting migration {}", self.version());
        run_sql_from_file(conn, &self.0.join("down.sql"))
    }
}

fn run_sql_from_file(conn: &Connection, path: &Path) -> Result<(), RunMigrationsError> {
    let mut sql = String::new();
    let mut file = try!(File::open(path));
    try!(file.read_to_string(&mut sql));
    try!(conn.batch_execute(&sql));
    Ok(())
}
