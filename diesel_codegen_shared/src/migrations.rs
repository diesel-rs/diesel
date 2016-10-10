use diesel::migrations::{search_for_migrations_directory, MigrationError};
use std::path::{PathBuf, Path};

pub fn resolve_migrations_directory(
    callsite_dir: &Path,
    relative_path_to_migrations: Option<&Path>,
) -> Result<PathBuf, MigrationError> {

    let result = match relative_path_to_migrations {
        Some(dir) => callsite_dir.join(dir),
        None => try!(search_for_migrations_directory(&callsite_dir)),
    };
    result.canonicalize().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use std::fs;
    use std::path::Path;
    use super::resolve_migrations_directory;

    #[test]
    fn migrations_directory_resolved_relative_to_callsite_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/special_migrations")).unwrap();
        let callsite = tempdir.path().join("foo");
        let relative_path = Some(Path::new("special_migrations"));

        assert_eq!(
            tempdir.path().join("foo/special_migrations").canonicalize().ok(),
            resolve_migrations_directory(&callsite, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_canonicalizes_result() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations/bar")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let callsite = tempdir.path().join("foo/bar/");
        let relative_path = Some(Path::new("../migrations/bar"));

        assert_eq!(
            tempdir.path().join("foo/migrations/bar").canonicalize().ok(),
            resolve_migrations_directory(&callsite, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_defaults_to_searching_migrations_path() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let callsite = tempdir.path().join("foo/bar/baz.rs");

        assert_eq!(
            tempdir.path().join("foo/migrations").canonicalize().ok(),
            resolve_migrations_directory(&callsite, None).ok()
        );
    }

    #[test]
    fn migrations_directory_allows_no_parent_dir_for_callsite() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("special_migrations")).unwrap();
        let callsite = tempdir.path();
        let relative_path = Some(Path::new("special_migrations"));
        assert_eq!(
            tempdir.path().join("special_migrations").canonicalize().ok(),
            resolve_migrations_directory(&callsite, relative_path).ok()
        );
    }
}
