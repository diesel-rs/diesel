use migrations_internals::search_for_migrations_directory;

use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

pub fn migration_directory_from_given_path(
    given_path: Option<&str>,
) -> Result<PathBuf, Box<Error>> {
    let cargo_toml_directory = env::var("CARGO_MANIFEST_DIR")?;
    let cargo_manifest_path = Path::new(&cargo_toml_directory);
    let migrations_path = given_path.as_ref().map(Path::new);
    resolve_migrations_directory(cargo_manifest_path, migrations_path)
}

fn resolve_migrations_directory(
    cargo_manifest_dir: &Path,
    relative_path_to_migrations: Option<&Path>,
) -> Result<PathBuf, Box<Error>> {
    let result = match relative_path_to_migrations {
        Some(dir) => cargo_manifest_dir.join(dir),
        None => {
            // People commonly put their migrations in src/migrations
            // so start the search there rather than the project root
            let src_dir = cargo_manifest_dir.join("src");
            search_for_migrations_directory(&src_dir)?
        }
    };

    result.canonicalize().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    extern crate tempdir;

    use self::tempdir::TempDir;
    use super::resolve_migrations_directory;
    use std::fs;
    use std::path::Path;

    #[test]
    fn migrations_directory_resolved_relative_to_cargo_manifest_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/special_migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo");
        let relative_path = Some(Path::new("special_migrations"));

        assert_eq!(
            tempdir
                .path()
                .join("foo/special_migrations")
                .canonicalize()
                .ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_canonicalizes_result() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations/bar")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo/bar");
        let relative_path = Some(Path::new("../migrations/bar"));

        assert_eq!(
            tempdir
                .path()
                .join("foo/migrations/bar")
                .canonicalize()
                .ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }

    #[test]
    fn migrations_directory_defaults_to_searching_migrations_path() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/bar")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo/bar");

        assert_eq!(
            tempdir.path().join("foo/migrations").canonicalize().ok(),
            resolve_migrations_directory(&cargo_manifest_dir, None).ok()
        );
    }

    #[test]
    fn migrations_directory_searches_src_migrations_if_present() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("foo/src/migrations")).unwrap();
        fs::create_dir_all(tempdir.path().join("foo/migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().join("foo");

        assert_eq!(
            tempdir
                .path()
                .join("foo/src/migrations")
                .canonicalize()
                .ok(),
            resolve_migrations_directory(&cargo_manifest_dir, None).ok()
        );
    }

    #[test]
    fn migrations_directory_allows_no_parent_dir_for_cargo_manifest_dir() {
        let tempdir = TempDir::new("diesel").unwrap();
        fs::create_dir_all(tempdir.path().join("special_migrations")).unwrap();
        let cargo_manifest_dir = tempdir.path().to_owned();
        let relative_path = Some(Path::new("special_migrations"));
        assert_eq!(
            tempdir
                .path()
                .join("special_migrations")
                .canonicalize()
                .ok(),
            resolve_migrations_directory(&cargo_manifest_dir, relative_path).ok()
        );
    }
}
