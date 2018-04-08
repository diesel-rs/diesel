use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Read;
use proc_macro::TokenStream;

use diesel::migration::{
    MigrationError, Migration, MigrationsDirectoryPlugin, RunMigrationsError
};
use diesel::connection::SimpleConnection;
use super::directory::MigrationsDirectory;

#[derive(Debug)]
pub struct SqlPlugin;

impl SqlPlugin {
    fn valid_directory(&self, path: &Path) -> bool {
        self.file_names(path)
            .map(|files| files.contains(&"down.sql".into()) && files.contains(&"up.sql".into()))
            .unwrap_or(false)
    }

    fn file_names(&self, path: &Path) -> Result<Vec<String>, MigrationError> {
        try!(path.read_dir())
            .map(|entry| {
                let file_name = try!(entry).file_name();

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
}

impl MigrationsDirectoryPlugin for SqlPlugin {
    fn load_migration_from_path(&self, path: &Path) -> Result<Box<Migration>, MigrationError> {
        if self.valid_directory(path) {
            let version = MigrationsDirectory::version_from_path(&path)?;
            let name = MigrationsDirectory::name_from_path(&path)?;
            Ok(Box::new(SqlFileMigration::new(path.into(), version, name)))
        } else {
            Err(MigrationError::UnknownMigrationFormat(path.into()))
        }
    }
}

#[derive(Debug)]
pub struct SqlFileMigration {
    path: PathBuf,
    version: String,
    name: String,
}

impl SqlFileMigration {
    pub fn new(path: PathBuf, version: String, name: String) -> Self {
        SqlFileMigration { path, version, name }
    }
    fn read_migration_sql(&self, name: &str) -> Result<String, MigrationError> {
        let path = self.path.join(name);

        let mut sql = String::new();
        let mut file = try!(File::open(path));
        try!(file.read_to_string(&mut sql));
        Ok(sql)
    }
    fn run_internal(&self, conn: &SimpleConnection, name: &str) -> Result<(), RunMigrationsError> {
        let sql = self.read_migration_sql(name)?;

        if sql.is_empty() {
            return Err(RunMigrationsError::EmptyMigration);
        }

        try!(conn.batch_execute(&sql));
        Ok(())
    }
}

impl Migration for SqlFileMigration {
    fn version(&self) -> &str {
        &self.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        self.run_internal(conn, "up.sql")
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        self.run_internal(conn, "down.sql")
    }

    fn file_path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    // Embed the migration as code (unstable)
    #[doc(hidden)]
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        (SqlEmbeddedMigration {
            version: &self.version,
            name: &self.name,
            up_sql: &self.read_migration_sql("up.sql")?,
            down_sql: &self.read_migration_sql("down.sql")?,
        }).embed()
    }
}

// Must be public so it can be used from the embedding macro
#[doc(hidden)]
#[derive(Debug)]
pub struct SqlEmbeddedMigration<'a> {
    pub version: &'a str,
    pub name: &'a str,
    pub up_sql: &'a str,
    pub down_sql: &'a str,
}

impl<'a> Migration for SqlEmbeddedMigration<'a> {
    fn version(&self) -> &str {
        self.version
    }

    fn name(&self) -> &str {
        self.name
    }

    fn run(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        try!(conn.batch_execute(self.up_sql));
        Ok(())
    }

    fn revert(&self, conn: &SimpleConnection) -> Result<(), RunMigrationsError> {
        try!(conn.batch_execute(self.down_sql));
        Ok(())
    }

    // Embed the migration as code (unstable)
    #[doc(hidden)]
    fn embed(&self) -> Result<TokenStream, MigrationError> {
        let version = self.version;
        let name = self.name;
        let up_sql = self.up_sql;
        let down_sql = self.down_sql;
        Ok(quote!(diesel_migrations::sql_plugin::SqlEmbeddedMigration {
            version: #version,
            name: #name,
            up_sql: #up_sql,
            down_sql: #down_sql,
        }).into())
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::*;

    use self::tempdir::TempDir;

    use std::fs;

    #[test]
    fn files_are_not_valid_sql_file_migrations() {
        let dir = TempDir::new("diesel").unwrap();
        let file_path = dir.path().join("12345");

        fs::File::create(&file_path).unwrap();

        assert!(!SqlPlugin.valid_directory(&file_path));
    }

    #[test]
    fn directory_containing_exactly_up_sql_and_down_sql_is_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();

        assert!(SqlPlugin.valid_directory(&folder));
    }

    #[test]
    fn directory_containing_unknown_files_is_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();
        fs::File::create(folder.join("foo")).unwrap();

        assert!(SqlPlugin.valid_directory(&folder));
    }

    #[test]
    fn files_beginning_with_dot_are_allowed() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();
        fs::File::create(folder.join("down.sql")).unwrap();
        fs::File::create(folder.join(".foo")).unwrap();

        assert!(SqlPlugin.valid_directory(&folder));
    }

    #[test]
    fn empty_directory_is_not_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();

        assert!(!SqlPlugin.valid_directory(&folder));
    }

    #[test]
    fn directory_with_only_up_sql_is_not_valid_migration_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        let folder = tempdir.path().join("12345");

        fs::create_dir(&folder).unwrap();
        fs::File::create(folder.join("up.sql")).unwrap();

        assert!(!SqlPlugin.valid_directory(&folder));
    }

}
