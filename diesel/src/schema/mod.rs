mod migration_error;

pub use self::migration_error::MigrationError;

use std::env;
use std::path::{PathBuf, Path};

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
