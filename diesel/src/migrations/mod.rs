mod migration;
mod migration_error;
mod schema;

pub use self::migration_error::*;
use self::migration::*;

use ::query_dsl::*;
use self::schema::NewMigration;
use self::schema::__diesel_schema_migrations::dsl::*;
use {Connection, QueryResult};

use std::collections::HashSet;
use std::env;
use std::path::{PathBuf, Path};

pub fn run_pending_migrations(conn: &Connection) -> Result<(), RunMigrationsError> {
    try!(create_schema_migrations_table_if_needed(conn));
    let already_run = try!(previously_run_migration_versions(conn));
    let migrations_dir = try!(find_migrations_directory());
    let all_migrations = try!(migrations_in_directory(&migrations_dir));
    let pending_migrations = all_migrations.into_iter().filter(|m| {
        !already_run.contains(&m.version())
    });
    run_migrations(conn, pending_migrations)
}

fn create_schema_migrations_table_if_needed(conn: &Connection) -> QueryResult<usize> {
    conn.execute("CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
        version INT8 PRIMARY KEY NOT NULL,
        run_on TIMESTAMP NOT NULL DEFAULT NOW()
    )")
}

fn previously_run_migration_versions(conn: &Connection) -> QueryResult<HashSet<i64>> {
    __diesel_schema_migrations.select(version)
        .load(&conn)
        .map(|r| r.collect())
}

fn migrations_in_directory(path: &Path) -> Result<Vec<Box<Migration>>, MigrationError> {
    use self::migration::migration_from;

    try!(path.read_dir())
        .map(|e| Ok(try!(e).path()))
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => return Some(Err(e)),
            };
            if entry.is_dir() {
                Some(migration_from(entry))
            } else {
                None
            }
        }).collect()
}

fn run_migrations<T>(conn: &Connection, migrations: T)
    -> Result<(), RunMigrationsError> where
        T: Iterator<Item=Box<Migration>>
{
    use ::query_builder::insert;

    for migration in migrations {
        try!(conn.transaction(|| {
            println!("Running migration {}", migration.version());
            try!(migration.run(conn));
            try!(insert(&NewMigration(migration.version()))
                 .into(__diesel_schema_migrations)
                 .execute(&conn));
            Ok(())
        }));
    }
    Ok(())
}

/// Returns the directory containing migrations. Will look at for
/// $PWD/migrations. If it is not found, it will search the parents
/// of the current directory, until it reaches the directory containing
/// `Cargo.toml`. Returns `MigrationError::MigrationDirectoryNotFound`
/// if no directory is found.
fn find_migrations_directory() -> Result<PathBuf, MigrationError> {
    search_for_migrations_directory(&try!(env::current_dir()))
}

fn search_for_migrations_directory(path: &Path) -> Result<PathBuf, MigrationError> {
    let migration_path = path.join("migrations");
    if migration_path.is_dir() {
        Ok(migration_path)
    } else if path.join("Cargo.toml").exists() {
        Err(MigrationError::MigrationDirectoryNotFound)
    } else {
        path.parent().map(search_for_migrations_directory)
            .unwrap_or(Err(MigrationError::MigrationDirectoryNotFound))
    }
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use super::*;
    use super::find_migrations_directory;

    use self::tempdir::TempDir;
    use std::{env, fs};

    #[test]
    fn migration_directory_not_found_if_no_migration_dir_exists() {
        let dir = TempDir::new("diesel").unwrap();

        env::set_current_dir(dir.path()).unwrap();

        assert_eq!(Err(MigrationError::MigrationDirectoryNotFound),
            find_migrations_directory());
    }

    #[test]
    fn migration_directory_defaults_to_pwd_slash_migrations() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path();
        let migrations_path = temp_path.join("migrations");

        env::set_current_dir(temp_path).unwrap();
        fs::create_dir(&migrations_path).unwrap();

        assert_eq!(Ok(migrations_path), find_migrations_directory());
    }

    #[test]
    fn migration_directory_checks_parents() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path();
        let migrations_path = temp_path.join("migrations");
        let child_path = temp_path.join("child");

        fs::create_dir(&child_path).unwrap();
        fs::create_dir(&migrations_path).unwrap();
        env::set_current_dir(child_path).unwrap();

        assert_eq!(Ok(migrations_path), find_migrations_directory());
    }

    #[test]
    fn migration_directory_stops_at_cargo_toml() {
        let dir = TempDir::new("diesel").unwrap();
        let temp_path = dir.path();
        let migrations_path = temp_path.join("migrations");
        let child_path = temp_path.join("child");
        let grandchild_path = child_path.join("grandchild");

        fs::create_dir_all(&grandchild_path).unwrap();
        fs::create_dir(&migrations_path).unwrap();
        fs::File::create(child_path.join("Cargo.toml")).unwrap();
        env::set_current_dir(grandchild_path).unwrap();

        assert_eq!(Err(MigrationError::MigrationDirectoryNotFound),
            find_migrations_directory());
    }
}
